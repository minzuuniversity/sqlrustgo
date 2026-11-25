import re, pathlib, json, sys, collections
root=pathlib.Path(sys.argv[1])
ATTR = re.compile(r'#\[ignore(\s*=\s*"((?:[^"\\]|\\[\s\S])*)")?\]')
BETWEEN = re.compile(r'(?:\s|//[^\n]*|/\*.*?\*/|\#[^\n]*)*?fn\s+([A-Za-z0-9_]+)')
tree=[]
for p in sorted(list((root/"tests").rglob("*.rs"))+list((root/"crates").rglob("*.rs"))):
    src=p.read_text(errors="replace")
    for m in ATTR.finditer(src):
        ls=src.rfind("\n",0,m.start())+1
        if src[ls:m.start()].strip().startswith("//"): continue
        b=BETWEEN.match(src[m.end():m.end()+400])
        if not b: continue
        tree.append((str(p.relative_to(root)), b.group(1), src.count("\n",0,m.start())+1))
reg=json.load(open(root/"tests/baseline/ignore_registry.json"))
R=collections.Counter((e["file"],e["test"]) for e in reg["ignored_tests"])
T=collections.Counter((f,t) for f,t,_ in tree)
print("tree_total",len(tree),"registry_entries",sum(R.values()),"total_allowed",reg["total_allowed"])
print("--- in tree, not in registry:")
for k in sorted((T-R).elements()):
    print("TREE_ONLY",k[0],k[1],[l for f,t,l in tree if (f,t)==k])
print("--- in registry, not in tree:")
for k in sorted((R-T).elements()):
    e=[x for x in reg["ignored_tests"] if (x["file"],x["test"])==k][0]
    print("REG_ONLY",k[0],k[1],e.get("line_at_capture"),e.get("status"))
