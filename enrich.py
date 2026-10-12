import json, re, inspect, traceback

# 用已安装的 akshare（venv）抽取每个函数的真实数据源 URL
import akshare as ak

data = json.load(open("akshare_interfaces.json", encoding="utf-8"))
URL_RE = re.compile(r'https?://[^\s"\'<>\)]+')
POST_HINT = re.compile(r'\.post\(|method=["\']post|requests\.post|http\.post', re.I)

ok = 0
fallback = 0
for item in data:
    name = item["name"]
    url = None
    method = "GET"
    try:
        fn = getattr(ak, name, None)
        if fn is None:
            raise AttributeError("not on ak namespace")
        src = inspect.getsource(fn)
        # 先找显式 url = "..." 赋值
        m = re.search(r'url\s*=\s*["\'](https?://[^"\']+)["\']', src)
        if m:
            url = m.group(1)
        else:
            # 退而求其次：源码里第一个 https 字面量（通常是接口基址）
            m2 = URL_RE.search(src)
            if m2:
                url = m2.group(0).rstrip('\\"\'')
        if POST_HINT.search(src[:4000]):
            method = "POST"
    except Exception:
        pass

    if not url:
        # 抽不到真实接口 -> 回退到官网该接口的文档页
        url = "https://akshare.akfamily.xyz/" + item["page"]
        method = "GET"
        fallback += 1
    else:
        ok += 1

    item["url"] = url
    item["method"] = method

print(f"real-url extracted={ok}, fallback-doc={fallback}, total={len(data)}")
json.dump(data, open("akshare_interfaces_enriched.json", "w", encoding="utf-8"),
          ensure_ascii=False, indent=2)
print("saved akshare_interfaces_enriched.json")
