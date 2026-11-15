#!/usr/bin/env python3
"""Compare React baseline captures with GPUI captures, pixel by pixel.

For each pair: share of differing pixels (any channel differs by more than
--tolerance), mean absolute error, a red heatmap, and a side-by-side sheet
(baseline | GPUI | diff). Writes tests/visual/report.html sorted by difference.
Requires Pillow only.

Usage: scripts/visual/compare-visual.py [--filter REGEX] [--tolerance 32]
"""
import argparse
import html
import json
import re
from pathlib import Path

from PIL import Image, ImageChops, ImageOps

ROOT = Path(__file__).resolve().parents[2]


def compare(before_path, after_path, diff_path, sheet_path, tolerance):
    before = Image.open(before_path).convert("RGB")
    after = Image.open(after_path).convert("RGB")
    if after.size != before.size:
        canvas = Image.new("RGB", before.size, (255, 0, 255))
        canvas.paste(after, (0, 0))
        after = canvas
    delta = ImageChops.difference(before, after)
    channels = delta.split()
    maximum = ImageChops.lighter(ImageChops.lighter(channels[0], channels[1]), channels[2])
    mask = maximum.point(lambda v: 255 if v > tolerance else 0)
    differing = mask.histogram()[255]
    total = before.size[0] * before.size[1]
    histogram = maximum.histogram()
    mae = sum(i * n for i, n in enumerate(histogram)) / total
    heat = Image.blend(ImageOps.grayscale(after).convert("RGB"), Image.new("RGB", before.size, (255, 255, 255)), 0.75)
    heat.paste((220, 30, 30), mask=mask)
    heat.save(diff_path)
    scale = 0.5
    w, h = int(before.size[0] * scale), int(before.size[1] * scale)
    sheet = Image.new("RGB", (w * 3 + 20, h), (255, 255, 255))
    for index, image in enumerate((before, after, heat)):
        sheet.paste(image.resize((w, h), Image.LANCZOS), (index * (w + 10), 0))
    sheet.save(sheet_path)
    return {"different_percent": round(100 * differing / total, 2), "mae": round(mae, 2)}


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--baseline", default="tests/visual/baseline")
    parser.add_argument("--current", default="tests/visual/current")
    parser.add_argument("--output", default="tests/visual/diff")
    parser.add_argument("--filter")
    parser.add_argument("--tolerance", type=int, default=32)
    args = parser.parse_args()
    baseline, current, output = ROOT / args.baseline, ROOT / args.current, ROOT / args.output
    output.mkdir(parents=True, exist_ok=True)
    pattern = re.compile(args.filter) if args.filter else None
    rows = []
    for after in sorted(current.glob("*.png")):
        if pattern and not pattern.search(after.name):
            continue
        before = baseline / after.name
        if not before.exists():
            continue
        result = compare(before, after, output / after.name, output / f"sheet-{after.name}", args.tolerance)
        rows.append({"file": after.name, **result})
    missing = sorted(
        p.name for p in baseline.glob("*.png")
        if not (current / p.name).exists() and (not pattern or pattern.search(p.name))
    )
    rows.sort(key=lambda r: -r["different_percent"])
    (output / "results.json").write_text(json.dumps({"tolerance": args.tolerance, "rows": rows, "missing": missing}, indent=2))
    report = [
        "<!doctype html><meta charset=utf-8><title>Computy visual comparison</title>",
        "<style>body{font:14px system-ui;margin:24px;background:#f6f8f6;color:#263d34}"
        "table{border-collapse:collapse}td,th{padding:6px 10px;border-bottom:1px solid #dde3da;text-align:left}"
        "img{max-width:100%;border:1px solid #dde3da;margin:8px 0 28px}h2{margin:28px 0 4px;font-size:15px}</style>",
        f"<h1>React baseline vs GPUI</h1><p>Tolerance per channel: {args.tolerance}/255. "
        f"{len(rows)} captures compared, {len(missing)} missing.</p>",
        "<table><tr><th>Capture</th><th>Different pixels</th><th>Mean abs. error</th></tr>",
    ]
    for row in rows:
        report.append(f"<tr><td><a href='#{html.escape(row['file'])}'>{html.escape(row['file'])}</a></td>"
                      f"<td>{row['different_percent']} %</td><td>{row['mae']}</td></tr>")
    report.append("</table>")
    for row in rows:
        name = html.escape(row["file"])
        report.append(f"<h2 id='{name}'>{name} — {row['different_percent']} %</h2>"
                      f"<p>Baseline (React) · GPUI · differences</p><img src='diff/sheet-{name}' loading=lazy>")
    if missing:
        report.append("<h2>Missing GPUI captures</h2><ul>" + "".join(f"<li>{html.escape(m)}</li>" for m in missing) + "</ul>")
    (output.parent / "report.html").write_text("\n".join(report))
    for row in rows[:40]:
        print(f"{row['different_percent']:6.2f} %  mae {row['mae']:5.2f}  {row['file']}")
    if missing:
        print(f"missing: {len(missing)}")


if __name__ == "__main__":
    main()
