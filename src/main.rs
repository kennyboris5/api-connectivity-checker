//! API 接口联通性测试器 —— Rust + egui 实现。
//!
//! 特点：参数（待测 URL）全部内置为示例，用户只需点击“开始测试”即可批量检测
//! 各接口的连通状况，无需手动输入任何参数。
//! 界面采用整宽堆叠式布局 + 文本自动换行，避免表格横向溢出被裁切；
//! 顶部提供按分类筛选的下拉框（纯内置选择，不要求手动输入）。

mod targets;
mod tester;

use eframe::egui;
use std::sync::mpsc::{self, Receiver};

use tester::{TestResult, TestStatus};

/// 分类筛选项（全部）
const FILTER_ALL: &str = "全部";

/// 应用状态。
struct App {
    /// 所有目标的当前结果（启动时即为“待测试”占位行）
    results: Vec<TestResult>,
    /// 是否正在测试
    running: bool,
    /// 结果回传通道的接收端（测试进行中持有，结束后置空）
    rx: Option<Receiver<TestResult>>,
    /// 上次开始测试的时间
    last_run: Option<String>,
    /// 当前分类筛选（"全部" 或某个分类名）
    filter: String,
    /// 所有分类列表（用于下拉筛选）
    categories: Vec<String>,
}

impl App {
    fn new() -> Self {
        let targets = targets::builtin_targets();
        let results = targets.iter().map(TestResult::pending).collect();

        // 由内置目标汇总去重出分类列表
        let mut categories: Vec<String> =
            targets.iter().map(|t| t.category.clone()).collect();
        categories.sort();
        categories.dedup();

        Self {
            results,
            running: false,
            rx: None,
            last_run: None,
            filter: FILTER_ALL.to_string(),
            categories,
        }
    }

    /// 启动一轮测试：重置结果、建立通道、开后台线程跑真实请求。
    fn start_test(&mut self, ctx: &egui::Context) {
        if self.running {
            return;
        }
        // 重置为待测试占位
        let init_targets = targets::builtin_targets();
        self.results = init_targets.iter().map(TestResult::pending).collect();

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.running = true;
        self.last_run = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());

        // 后台线程执行真实联网测试（不阻塞 GUI）
        let test_targets = targets::builtin_targets();
        std::thread::spawn(move || {
            tester::run_tests(test_targets, tx);
        });
        ctx.request_repaint();
    }

    /// 统计当前筛选下显示的数量，以及其中成功/失败数。
    fn stats(&self) -> (usize, usize, usize) {
        let mut shown = 0;
        let mut ok = 0;
        let mut fail = 0;
        for r in &self.results {
            if self.filter != FILTER_ALL && r.category != self.filter {
                continue;
            }
            shown += 1;
            match r.status {
                TestStatus::Ok => ok += 1,
                TestStatus::Failed => fail += 1,
                _ => {}
            }
        }
        (shown, ok, fail)
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1) 收集中后台线程回传的结果，就地更新对应行
        if let Some(rx) = &self.rx {
            while let Ok(r) = rx.try_recv() {
                if let Some(slot) = self.results.iter_mut().find(|x| x.id == r.id) {
                    *slot = r;
                }
            }
        }
        // 2) 所有目标都不再是“待测试/测试中”即本轮结束
        if self.running
            && self
                .results
                .iter()
                .all(|r| r.status != TestStatus::Pending && r.status != TestStatus::Running)
        {
            self.running = false;
            self.rx = None;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("API 接口联通性测试器");
            ui.label("参数（URL）已内置为示例，点击按钮即可批量检测各接口连通状况，无需手动输入。");

            // 顶部操作栏：开始按钮 + 上次时间 + 分类筛选 + 统计
            ui.horizontal_wrapped(|ui| {
                let btn = ui.button(if self.running {
                    "测试中…（请稍候）"
                } else {
                    "▶ 开始测试 (内置示例)"
                });
                if btn.clicked() {
                    self.start_test(ctx);
                }
                if let Some(t) = &self.last_run {
                    ui.label(format!("上次测试：{t}"));
                }

                // 分类筛选下拉（纯内置选择）
                egui::ComboBox::from_label("分类筛选")
                    .selected_text(self.filter.clone())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.filter, FILTER_ALL.to_string(), FILTER_ALL);
                        for c in &self.categories {
                            ui.selectable_value(&mut self.filter, c.clone(), c.as_str());
                        }
                    });

                let (shown, ok, fail) = self.stats();
                let total = self.results.len();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "成功 {ok} / 失败 {fail} / 显示 {shown} / 共 {total}"
                    ));
                });
            });

            ui.separator();

            // 结果区：整宽堆叠式，长 URL 自动换行，杜绝横向裁切
            egui::ScrollArea::vertical().show(ui, |ui| {
                // 让区间内所有文本按可用宽度自动换行
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                for r in &self.results {
                    if self.filter != FILTER_ALL && r.category != self.filter {
                        continue;
                    }

                    // 状态 / 名称 / HTTP / 延迟 / 大小 一行
                    let (label, color) = match r.status {
                        TestStatus::Ok => ("✓ 可达", egui::Color32::from_rgb(46, 160, 67)),
                        TestStatus::Failed => ("✗ 失败", egui::Color32::from_rgb(207, 34, 34)),
                        TestStatus::Running => ("… 测试中", egui::Color32::from_rgb(200, 160, 0)),
                        TestStatus::Pending => ("· 待测试", egui::Color32::from_rgb(140, 140, 140)),
                    };
                    ui.horizontal(|ui| {
                        ui.colored_label(color, label);
                        ui.strong(&r.name);
                        ui.separator();
                        match r.http_code {
                            Some(c) => ui.label(format!("HTTP {c}")),
                            None => ui.label("HTTP -"),
                        };
                        match r.latency_ms {
                            Some(ms) => ui.label(format!("延迟 {ms} ms")),
                            None => ui.label("延迟 -"),
                        };
                        match r.size_bytes {
                            Some(s) => ui.label(format!("大小 {s} B")),
                            None => ui.label("大小 -"),
                        };
                        ui.separator();
                        ui.label(egui::RichText::new(&r.method).weak().small());
                    });

                    // 分类 + 用途说明
                    ui.label(
                        egui::RichText::new(format!("分类：{}　|　{}", r.category, r.description))
                            .small()
                            .weak(),
                    );

                    // URL（自动换行，避免溢出）
                    ui.label(egui::RichText::new(&r.url).monospace());

                    // 错误 / 响应预览
                    if let Some(e) = &r.error {
                        ui.colored_label(
                            egui::Color32::from_rgb(207, 34, 34),
                            format!("错误：{e}"),
                        );
                    } else if !r.body_preview.is_empty() {
                        ui.label(egui::RichText::new(format!("响应预览：{}", r.body_preview)));
                    }

                    ui.separator();
                }
            });
        });
    }
}

/// 尝试加载系统中的中文字体，保证界面中文正常显示。
/// 若所有候选字体都读不到，则退回 egui 默认字体（此时中文可能显示为方块）。
fn setup_fonts(ctx: &egui::Context) {
    let candidates = [
        "C:/Windows/Fonts/msyh.ttc", // 微软雅黑（Win10/11 常见，TTC 合集）
        "C:/Windows/Fonts/msyh.ttf",
        "C:/Windows/Fonts/simhei.ttf", // 黑体
        "C:/Windows/Fonts/simsun.ttc", // 宋体（TTC 合集）
        "C:/Windows/Fonts/NotoSansCJK-Regular.ttc",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("cjk".to_owned(), egui::FontData::from_owned(bytes));
            if let Some(proportional) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                proportional.insert(0, "cjk".to_owned());
            }
            if let Some(mono) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                mono.insert(0, "cjk".to_owned());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 680.0])
            .with_title("API 接口联通性测试器"),
        ..Default::default()
    };

    eframe::run_native(
        "API 接口联通性测试器",
        options,
        Box::new(|cc: &eframe::CreationContext| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(App::new()) as Box<dyn eframe::App>)
        }),
    )
}
