#!/usr/bin/env python3
"""Pair light and dark computed colors per element for a captured scenario.
Usage: scripts/visual/theme-diff.py tests/visual/reference/styles/overview"""
import re, sys
base = sys.argv[1]
light = open(f"{base}-light-fr.txt").read().splitlines()
dark = open(f"{base}-dark-fr.txt").read().splitlines()
keys = ("bg", "color", "border", "shadow", "bgimg", "opacity")
def attrs(line):
    out = {}
    for key in keys:
        m = re.search(rf" {key}=((?:[^ ]+(?: [0-9.-]+px)*)+?)(?= [a-z]+=|\s\"|$)", line)
        if m:
            out[key] = m.group(1)
    return out
for l, d in zip(light, dark):
    name = l.strip().split(" ")[0]
    text = re.search(r'"(.*)"$', l)
    a, b = attrs(l), attrs(d)
    pairs = [f"{k}: {a.get(k,'-')} → {b.get(k,'-')}" for k in keys if a.get(k) != b.get(k) or k in a]
    if pairs:
        print(f"{name[:48]:48} {(text.group(1)[:24] if text else ''):24} | " + "; ".join(pairs))
