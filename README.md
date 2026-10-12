# API 接口联通性测试器（Rust + egui）

一个用 **Rust + egui** 写的桌面小工具：一键并发检测大量数据接口的联通状况，参数全部内置为示例，**无需用户手动输入任何内容**。

> 最初需求来自 AKShare（开源财经数据接口库）。本项目把 AKShare 官网文档列出的全部接口真实爬取下来，逐条附带其真实数据源 URL，做成可视化联通性巡检工具。可直接用来排查「哪些数据接口在本机/本网络环境下能通、哪些被限流/重置」。

## 功能特性

- **内置 1018 个接口**（从 AKShare 官网 `akshare.akfamily.xyz` 真实爬取，覆盖股票 / 港股 / 美股 / ETF / 基金 / 指数 / 期货 / 期权 / 外汇 / 债券 / 宏观 / 数字货币 / 现货 / 新闻研报 / 各交易所官网 等全品类）。
- 每个接口带**真实数据源 URL**（从本地 `akshare` 源码反向抽取，共 1009 条命中真实端点，其余回退到官方文档页）。
- 点「开始测试」即**并发**发起 HTTP 请求，结果实时刷新。
- 每条显示：状态（✓ 可达 / ✗ 失败 / 测试中）、HTTP 状态码、延迟、响应大小、响应体预览 / 错误。
- **HTTP 4xx/5xx 仍算「已连到服务器 = 可达」**；只有连接超时、被对端重置、DNS 失败才算失败。
- 顶部分类下拉框，可按品类筛选（纯内置选择，不要求输入）。
- 自动加载系统 `msyh`（微软雅黑）中文字体，**中文不乱码**。

## 构建

依赖已在本机全局缓存，正常联网环境直接：

```bash
cd api-connectivity-checker
cargo run                 # 调试运行
cargo build --release     # 产出 Release 可执行文件
```

产物：`target/release/api-connectivity-checker.exe`（Windows x64）。

> 本项目 GUI 后端显式选用 `eframe` 的 `glow`（OpenGL）后端；HTTP 客户端用 `ureq`（同步、极小、无异步运行时依赖）。在「无法访问 crate 仓库」的受限网络下也能离线编译。

## 使用

1. 启动后，默认加载全部 1018 个内置接口。
2. 可选：用顶部分类下拉框筛选某一类接口（如只看「股票」）。
3. 点击「开始测试」，等待并发请求结束。
4. 结果区逐条展示状态 / 状态码 / 延迟 / 大小 / 预览。顶部统计「成功 / 失败 / 共」。

## 项目结构

```
api-connectivity-checker/
├── Cargo.toml
├── src/
│   ├── main.rs        # egui 界面 + 中文字体加载
│   ├── tester.rs      # 并发连通性测试逻辑
│   └── targets.rs     # 内置 1018 个接口清单（由脚本生成，勿手改）
├── crawl_interfaces.py  # 爬 AKShare 官网，抽取真实接口名 + 分类
├── enrich.py           # 从 akshare 源码抽取每个接口的真实数据源 URL
└── gen_targets.py      # 把上两步结果生成 targets.rs
```

## 复现接口清单（可选）

如需重新爬取 / 更新内置接口：

```bash
pip install akshare        # enrich.py 依赖
python crawl_interfaces.py # 输出 akshare_interfaces.json
python enrich.py           # 输出 akshare_interfaces_enriched.json
python gen_targets.py      # 重新生成 src/targets.rs
```

## 技术栈

| 组件 | 选型 | 说明 |
|------|------|------|
| 语言 | Rust (edition 2021) | |
| GUI | egui / eframe 0.29 (glow 后端) | 原生窗口 |
| HTTP | ureq 2 | 同步阻塞、原生 TLS |
| 时间 | chrono 0.4 | |

## 许可

MIT License。详见 [LICENSE](./LICENSE)。
