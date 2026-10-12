//! 连通性测试核心逻辑。
//!
//! 对每个内置目标发起 HTTP GET 请求，记录状态码、延迟、响应大小与响应体预览，
//! 并通过 `mpsc` 通道把每个目标的结果实时回传给 GUI 线程（便于界面逐步刷新）。
//!
//! 使用 `ureq`（极简阻塞式 HTTP 客户端，无需异步运行时）。HTTP 错误状态码（4xx/5xx）
//! 仍表示“已连到服务器”，按“可达”处理；只有连接失败/超时/DNS 失败才算“失败”。

use crate::targets::Target;
use chrono::Local;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// 单个目标的测试状态。
#[derive(Clone, PartialEq, Eq)]
pub enum TestStatus {
    /// 尚未开始
    Pending,
    /// 正在请求中
    Running,
    /// 请求成功（拿到响应，含 4xx/5xx）
    Ok,
    /// 请求失败（超时 / 连接被拒 / DNS 失败等）
    Failed,
}

/// 单个目标的测试结果。
#[derive(Clone)]
pub struct TestResult {
    /// 目标唯一标识（用名称做 id）
    pub id: String,
    /// 展示名称
    pub name: String,
    /// 请求 URL
    pub url: String,
    /// 用途说明
    pub description: String,
    /// 请求方法（GET / POST）
    pub method: String,
    /// 所属分类
    pub category: String,
    /// 当前状态
    pub status: TestStatus,
    /// HTTP 状态码（成功时存在）
    pub http_code: Option<u16>,
    /// 请求耗时（毫秒）
    pub latency_ms: Option<u128>,
    /// 响应体字节数
    pub size_bytes: Option<usize>,
    /// 响应体前若干字符预览
    pub body_preview: String,
    /// 失败原因
    pub error: Option<String>,
    /// 完成测试的时间点（本地 时:分:秒）
    pub tested_at: Option<String>,
}

impl TestResult {
    /// 由目标生成一个“待测试”的占位结果。
    pub fn pending(t: &Target) -> Self {
        Self {
            id: t.name.clone(),
            name: t.name.clone(),
            url: t.url.clone(),
            description: t.description.clone(),
            method: t.method.clone(),
            category: t.category.clone(),
            status: TestStatus::Pending,
            http_code: None,
            latency_ms: None,
            size_bytes: None,
            body_preview: String::new(),
            error: None,
            tested_at: None,
        }
    }
}

/// 当前本地时间（时:分:秒）。
fn now_str() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

/// 对全部目标并发发起请求，每完成一个就通过 `tx` 发送一次最新结果。
///
/// 使用阻塞式 `ureq` + 线程作用域（scoped threads），每个目标一个线程，
/// 共享同一个带超时的 `Agent`。超时设为 10 秒，避免某个卡死的目标拖住整体。
pub fn run_tests(targets: Vec<Target>, tx: Sender<TestResult>) {
    let agent = Arc::new(
        ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(10))
            .build(),
    );

    thread::scope(|s| {
        for t in targets {
            let agent = Arc::clone(&agent);
            let tx = tx.clone();
            s.spawn(move || {
                let mut res = TestResult::pending(&t);
                // 先发“测试中”状态，让界面立刻有反馈
                res.status = TestStatus::Running;
                let _ = tx.send(res.clone());

                let start = Instant::now();
                let request = match t.method.as_str() {
                    "POST" => agent.post(&t.url),
                    _ => agent.get(&t.url),
                }
                .set(
                    "User-Agent",
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) APIConnectivityChecker/0.1",
                )
                // 新浪等部分接口会校验 Referer，统一带上以通过
                .set("Referer", "https://finance.sina.com.cn/");

                match request.call() {
                    Ok(resp) => {
                        // 2xx：正常响应
                        let code = resp.status();
                        let elapsed = start.elapsed();
                        finalize_from_body(resp.into_string(), code, elapsed, &mut res);
                    }
                    Err(ureq::Error::Status(code, resp)) => {
                        // 非 2xx：仍连到了服务器，按“可达”处理并展示状态码
                        let elapsed = start.elapsed();
                        finalize_from_body(resp.into_string(), code, elapsed, &mut res);
                    }
                    Err(e) => {
                        // 连接失败 / 超时 / DNS 失败等
                        res.latency_ms = Some(start.elapsed().as_millis());
                        res.status = TestStatus::Failed;
                        res.error = Some(e.to_string());
                        res.tested_at = Some(now_str());
                    }
                }
                // 发送最终结果
                let _ = tx.send(res);
            });
        }
    });
}

/// 读取响应体并填充结果字段（被 2xx 与 4xx/5xx 两路复用）。
fn finalize_from_body(
    body: Result<String, std::io::Error>,
    code: u16,
    elapsed: Duration,
    res: &mut TestResult,
) {
    res.http_code = Some(code);
    res.latency_ms = Some(elapsed.as_millis());
    res.status = TestStatus::Ok; // 已连到服务器
    res.tested_at = Some(now_str());
    match body {
        Ok(body) => {
            res.size_bytes = Some(body.len());
            res.body_preview = body.chars().take(160).collect();
        }
        Err(e) => {
            res.error = Some(format!("读取响应体失败: {e}"));
        }
    }
}
