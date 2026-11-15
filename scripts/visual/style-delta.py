#!/usr/bin/env python3
"""Show style properties that differ between two dumps of the same page
(e.g. 1440 px vs 1920 px), ignoring positions. Lines are paired in order."""
import re, sys
a = open(sys.argv[1]).read().splitlines()
b = open(sys.argv[2]).read().splitlines()
def props(line):
    line = re.sub(r"\[[^\]]*\]", "", line)
    text = re.search(r'"(.*)"$', line)
    line = re.sub(r'"(.*)"$', "", line)
    parts = line.split()
    return parts[0] if parts else "", set(parts[1:]), text.group(1)[:30] if text else ""
import difflib
key = lambda line: (line.strip().split(" ")[0], props(line)[2])
matcher = difflib.SequenceMatcher(a=[key(l) for l in a], b=[key(l) for l in b], autojunk=False)
pairs = []
for tag, i1, i2, j1, j2 in matcher.get_opcodes():
    if tag == "equal":
        pairs.extend(zip(a[i1:i2], b[j1:j2]))
    else:
        for line in a[i1:i2]:
            print("ONLY-A", line.strip()[:110])
        for line in b[j1:j2]:
            print("ONLY-B", line.strip()[:110])
for x, y in pairs:
    na, pa, ta = props(x)
    nb, pb, tb = props(y)
    da, db = pa - pb, pb - pa
    da = {p for p in da if not p.startswith(("cols=",))}
    db = {p for p in db if not p.startswith(("cols=",))}
    if da or db:
        print(f"{x.strip().split(' ')[0][:40]:40} {ta:30} {' '.join(sorted(da))}  →  {' '.join(sorted(db))}")
