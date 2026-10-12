import json

data = json.load(open("akshare_interfaces_enriched.json", encoding="utf-8"))

def esc(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')

def clean_cat(c: str) -> str:
    c = c.replace("AKShare", "").replace("akshare", "")
    c = c.replace("1.19.1", "").replace("文档", "")
    c = c.replace("—", "").replace("–", "").replace("-", " ")
    return c.strip(" -|　")


def clean_desc(s: str) -> str:
    # 去掉 Sphinx 标题锚点图标（U+F0C1 / U+00B6）
    return s.replace("\uf0c1", "").replace("\u00b6", "").strip()

lines = []
fallback = 0
for it in data:
    name = it["name"]
    cat = clean_cat(it.get("category", "")) or "未分类"
    desc = clean_desc(it.get("description", "") or "")
    url = it.get("url", "")
    method = it.get("method", "GET") or "GET"
    # 模板化 URL 回退到官网该接口文档页
    if "{" in url or "<" in url or not url.startswith("http"):
        url = "https://akshare.akfamily.xyz/" + it.get("page", "")
        method = "GET"
        fallback += 1
    lines.append(
        f'        Target::new("{esc(name)}", "{esc(url)}", "{esc(cat)}", "{esc(desc)}", "{esc(method)}"),'
    )

header = '''//! 内置的 API 测试目标清单（覆盖 AKShare 全部数据品类接口）。
//!
//! 本清单由官方文档站（akshare.akfamily.xyz）真实爬取得到，共 1000+ 接口，
//! 非人工猜测。每个接口的「真实数据源 URL」从本地已安装的 akshare 源码抽取；
//! 模板化（参数化）接口则回退到其官方文档页。参数均已内置，用户无需手动输入。

/// 一个待测的 API 目标。
#[derive(Clone)]
pub struct Target {
    /// 展示名称（界面左侧显示）
    pub name: String,
    /// 完整请求 URL（已内置示例标的/参数）
    pub url: String,
    /// 用途说明
    pub description: String,
    /// 请求方法（GET / POST）
    pub method: String,
    /// 所属分类（用于筛选与展示）
    pub category: String,
}

impl Target {
    pub fn new(
        name: &str,
        url: &str,
        category: &str,
        description: &str,
        method: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            url: url.to_string(),
            description: description.to_string(),
            method: method.to_string(),
            category: category.to_string(),
        }
    }
}

/// 返回内置的全部测试目标（真实爬取自 AKShare 官方文档，覆盖各品类）。
pub fn builtin_targets() -> Vec<Target> {
    vec![
'''

footer = """
    ]
}
"""

body = header + "\n".join(lines) + footer
with open("src/targets.rs", "w", encoding="utf-8") as f:
    f.write(body)

print(f"generated src/targets.rs with {len(lines)} targets (templated->doc fallback={fallback})")
