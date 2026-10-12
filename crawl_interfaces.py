import zlib, re, json, urllib.request, concurrent.futures, html

BASE = "https://akshare.akfamily.xyz/"

with open("objects.inv", "rb") as f:
    raw = f.read()
i = 0
for line in raw.split(b"\n"):
    if line.startswith(b"#"):
        i += 1
    else:
        break
comp = b"\n".join(raw.split(b"\n")[i:])
txt = zlib.decompress(comp).decode("utf-8", "replace")
pages = []
for l in txt.splitlines():
    p = l.split(" ")
    if len(p) >= 4 and p[1] == "std:doc" and p[3].startswith("data/"):
        pages.append(p[3])

FUNC_RE = re.compile(r"接口[:：]\s*([A-Za-z_][A-Za-z0-9_]*)")
HEAD_RE = re.compile(r"<h[1-6][^>]*>(.*?)</h[1-6]>", re.S)
TAG_RE = re.compile(r"<[^>]+>")


def clean(s):
    return html.unescape(re.sub(TAG_RE, "", s)).strip()


def fetch(page):
    url = BASE + page
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
        with urllib.request.urlopen(req, timeout=40) as r:
            body = r.read().decode("utf-8", "replace")
    except Exception as e:
        return {"page": page, "url": url, "title": "", "error": str(e), "items": []}

    # 页面分类名（去掉 "AKShare"/"文档" 后缀）
    m = re.search(r"<title>(.*?)</title>", body, re.S)
    cat = clean(m.group(1)) if m else page
    cat = cat.replace("AKShare", "").replace("—", "").replace("–", "").strip(" -|")

    funcs = []
    seen = set()
    for fm in FUNC_RE.finditer(body):
        name = fm.group(1)
        if name in seen:
            continue
        seen.add(name)
        pos = fm.start()
        # 向前找最近的中文标题（h1-h6），作为该接口的用途说明
        head = ""
        hm = None
        for h in HEAD_RE.finditer(body[:pos]):
            hm = h
        if hm:
            head = clean(hm.group(1))
            head = re.sub(r"¶.*$", "", head).strip()
        funcs.append({"name": name, "description": head})
    return {"page": page, "url": url, "title": cat, "error": None, "items": funcs}


results = []
with concurrent.futures.ThreadPoolExecutor(max_workers=8) as ex:
    for r in ex.map(fetch, pages):
        results.append(r)
        n = len(r["items"])
        print(f"  {r['page']:38s} interfaces={n}  cat={r['title']}  {'ERR:'+str(r['error']) if r['error'] else ''}")

# 扁平化
flat = []
for r in results:
    for it in r["items"]:
        flat.append({
            "name": it["name"],
            "description": it["description"],
            "category": r["title"],
            "page": r["page"],
        })

# 去重（按 name 保留首次出现）
uniq = {}
for f in flat:
    if f["name"] not in uniq:
        uniq[f["name"]] = f
flat = list(uniq.values())

print("TOTAL unique interfaces:", len(flat))
with open("akshare_interfaces.json", "w", encoding="utf-8") as f:
    json.dump(flat, f, ensure_ascii=False, indent=2)
print("saved akshare_interfaces.json")
