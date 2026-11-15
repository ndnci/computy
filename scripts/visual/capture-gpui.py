#!/usr/bin/env python3
"""Capture the GPUI interface for every scenario of the React baseline.

Runs inside the GPUI toolchain container (scripts/gpui-docker.sh) with a
headless sway compositor and grim. Each scenario launches
`computy --capture <scenario>`; scroll steps are driven over stdin.

Usage: scripts/gpui-docker.sh python3 scripts/visual/capture-gpui.py \
          [--binary desktop/target/debug/computy] [--filter REGEX]
"""
import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def start_sway(runtime):
    config = runtime / "sway.conf"
    config.write_text(
        "output HEADLESS-1 resolution 1440x960 scale 1\n"
        "default_border none\ndefault_floating_border none\n"
        "seat * hide_cursor 1\n"
    )
    env = os.environ | {
        "XDG_RUNTIME_DIR": str(runtime),
        "WLR_BACKENDS": "headless",
        "WLR_LIBINPUT_NO_DEVICES": "1",
        "WLR_RENDERER": "pixman",
    }
    sway = subprocess.Popen(
        ["sway", "--unsupported-gpu", "-c", str(config)],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    for _ in range(100):
        sockets = [p.name for p in runtime.iterdir() if re.fullmatch(r"wayland-\d+", p.name)]
        if sockets:
            env["WAYLAND_DISPLAY"] = sockets[0]
            env["SWAYSOCK"] = next(str(p) for p in runtime.iterdir() if p.name.startswith("sway-ipc"))
            return sway, env
        time.sleep(0.1)
    raise SystemExit("sway did not start")


def read_ready(process, timeout=30):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        line = process.stdout.readline()
        if not line:
            if process.poll() is not None:
                return None
            continue
        if line.startswith("ready"):
            return line.strip()
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="desktop/target/debug/computy")
    parser.add_argument("--baseline", default="tests/visual/baseline")
    parser.add_argument("--output", default="tests/visual/current")
    parser.add_argument("--filter")
    args = parser.parse_args()
    manifest = json.loads((ROOT / args.baseline / "manifest.json").read_text())
    output = ROOT / args.output
    output.mkdir(parents=True, exist_ok=True)
    runtime = Path(tempfile.mkdtemp(prefix="computy-sway-"))
    runtime.chmod(0o700)
    sway, env = start_sway(runtime)
    env |= {"TZ": "Europe/Paris", "COMPUTY_FIXTURE": str(ROOT / "tests/visual/fixture.json")}
    pattern = re.compile(args.filter) if args.filter else None
    results = []
    try:
        for scenario in manifest:
            if pattern and not any(pattern.search(f) for f in scenario["files"]):
                continue
            width, height = scenario["width"], scenario["height"]
            subprocess.run(
                ["swaymsg", "output", "HEADLESS-1", "resolution", f"{width}x{height}"],
                env=env,
                stdout=subprocess.DEVNULL,
                check=True,
            )
            spec = {k: scenario[k] for k in ("id", "actions", "setup", "theme", "locale") if k in scenario}
            spec |= {"page": scenario.get("page", "overview"), "width": width, "height": height}
            if scenario.get("popover"):
                spec["popover"] = scenario["popover"]
            process = subprocess.Popen(
                [str(ROOT / args.binary), "--capture", json.dumps(spec)],
                env=env,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=1,
            )
            captured = []
            ok = read_ready(process) is not None
            for step, name in enumerate(scenario["files"]):
                if not ok:
                    break
                if step:
                    process.stdin.write(f"scroll {step * 860}\n")
                    process.stdin.flush()
                    ok = read_ready(process) is not None
                    if not ok:
                        break
                subprocess.run(["grim", "-g", f"0,0 {width}x{height}", str(output / name)], env=env, check=True)
                captured.append(name)
            try:
                process.stdin.write("quit\n")
                process.stdin.flush()
                process.wait(5)
            except Exception:
                process.kill()
            errors = process.stderr.read()[-2000:] if not ok else ""
            results.append({"id": scenario["id"], "theme": scenario["theme"], "locale": scenario["locale"], "files": captured, "ok": ok, "errors": errors})
            print(f"{'ok ' if ok else 'ERR'} {scenario['id']} {scenario['theme']} {scenario['locale']} {len(captured)}/{len(scenario['files'])}", flush=True)
            if not ok:
                print(errors, file=sys.stderr)
    finally:
        sway.terminate()
    (output / "manifest.json").write_text(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
