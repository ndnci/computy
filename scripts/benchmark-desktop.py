#!/usr/bin/env python3
"""Measure Computy's desktop resource use on Linux.

The whole process tree is measured (WebKit helper processes included). Each
scenario starts the binary, waits for start-up to settle, then samples memory,
CPU and threads. Results are written as JSON for docs/BENCHMARK.md.

Usage:
  scripts/benchmark-desktop.py --binary /usr/bin/computy --label tauri-0.2.0 \
      --output docs/benchmarks/0.2.0-tauri.json
"""
import argparse
import json
import os
import platform
import signal
import shutil
import statistics
import subprocess
import tempfile
import time
from pathlib import Path

CLOCK = os.sysconf("SC_CLK_TCK")
PAGE = os.sysconf("SC_PAGE_SIZE")


def children_map():
    parents = {}
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            stat = (entry / "stat").read_text()
        except OSError:
            continue
        fields = stat[stat.rfind(")") + 2:].split()
        parents.setdefault(int(fields[1]), []).append(int(entry.name))
    return parents


def tree(pid):
    parents = children_map()
    result, stack = [], [pid]
    while stack:
        current = stack.pop()
        result.append(current)
        stack.extend(parents.get(current, []))
    return result


def process_sample(pid):
    base = Path(f"/proc/{pid}")
    try:
        rollup = {}
        for line in (base / "smaps_rollup").read_text().splitlines()[1:]:
            name, value = line.split(":", 1)
            rollup[name] = int(value.split()[0])
        stat = (base / "stat").read_text()
        fields = stat[stat.rfind(")") + 2:].split()
        status = (base / "status").read_text()
        threads = int(next(l for l in status.splitlines() if l.startswith("Threads:")).split()[1])
        name = (base / "comm").read_text().strip()
    except (OSError, StopIteration):
        return None
    return {
        "pid": pid,
        "name": name,
        "pss_kib": rollup.get("Pss", 0),
        "rss_kib": rollup.get("Rss", 0),
        "uss_kib": rollup.get("Private_Clean", 0) + rollup.get("Private_Dirty", 0),
        "cpu_ticks": int(fields[11]) + int(fields[12]),
        "threads": threads,
    }


def snapshot(pid):
    samples = [s for s in (process_sample(p) for p in tree(pid)) if s]
    return {
        "time": time.monotonic(),
        "processes": samples,
        "pss_kib": sum(s["pss_kib"] for s in samples),
        "rss_kib": sum(s["rss_kib"] for s in samples),
        "uss_kib": sum(s["uss_kib"] for s in samples),
        "cpu_ticks": sum(s["cpu_ticks"] for s in samples),
        "threads": sum(s["threads"] for s in samples),
    }


def application_pid(launcher, binary):
    """The measured tree starts at the application, not at dbus-run-session."""
    for pid in tree(launcher):
        try:
            if os.readlink(f"/proc/{pid}/exe") == binary:
                return pid
        except OSError:
            continue
    return None


def run(binary, arguments, settle, duration, interval, environment):
    # A private session bus keeps single-instance checks and tray registration
    # away from a Computy instance already running for the user.
    process = subprocess.Popen(
        ["dbus-run-session", "--", binary, *arguments],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        env=environment,
        start_new_session=True,
    )
    started = time.monotonic()
    try:
        time.sleep(settle)
        if process.poll() is not None:
            raise SystemExit(f"{binary} exited during start-up ({process.returncode})")
        pid = application_pid(process.pid, binary)
        if pid is None:
            raise SystemExit(f"{binary} is not running after start-up")
        first = snapshot(pid)
        series = [first]
        while time.monotonic() - started < settle + duration:
            time.sleep(interval)
            series.append(snapshot(pid))
        last = series[-1]
        elapsed = last["time"] - first["time"]
        cpu = 100 * (last["cpu_ticks"] - first["cpu_ticks"]) / CLOCK / elapsed if elapsed else 0
        return {
            "pss_mib": round(statistics.median(s["pss_kib"] for s in series) / 1024, 1),
            "rss_mib": round(statistics.median(s["rss_kib"] for s in series) / 1024, 1),
            "uss_mib": round(statistics.median(s["uss_kib"] for s in series) / 1024, 1),
            "peak_pss_mib": round(max(s["pss_kib"] for s in series) / 1024, 1),
            "cpu_percent": round(cpu, 2),
            "threads": last["threads"],
            "processes": len(last["processes"]),
            "per_process": [
                {"name": p["name"], "pss_mib": round(p["pss_kib"] / 1024, 1), "threads": p["threads"]}
                for p in sorted(last["processes"], key=lambda p: -p["pss_kib"])
            ],
        }
    finally:
        os.killpg(process.pid, signal.SIGTERM)
        try:
            process.wait(10)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()


def summarize(runs):
    keys = ["pss_mib", "rss_mib", "uss_mib", "peak_pss_mib", "cpu_percent", "threads", "processes"]
    return {
        key: {
            "median": round(statistics.median(r[key] for r in runs), 2),
            "min": min(r[key] for r in runs),
            "max": max(r[key] for r in runs),
        }
        for key in keys
    } | {"per_process": runs[len(runs) // 2]["per_process"]}


def gpu_names():
    try:
        output = subprocess.run(["lspci"], capture_output=True, text=True).stdout
    except OSError:
        return []
    return [l.split(": ", 1)[1] for l in output.splitlines() if "VGA" in l or "3D controller" in l]


def public_path(binary):
    """Path recorded in the published JSON, without the contributor's home directory."""
    path = Path(binary)
    root = Path(__file__).resolve().parent.parent
    if path.is_relative_to(root):
        return str(path.relative_to(root))
    if path.is_relative_to(Path.home()):
        return path.name
    return str(path)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--binary", required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--settle", type=float, default=15)
    parser.add_argument("--duration", type=float, default=60)
    parser.add_argument("--interval", type=float, default=1)
    parser.add_argument("--package", type=Path, help="Installer to report the size of")
    parser.add_argument("--reports", type=Path, help="Report directory copied into each run's data directory")
    parser.add_argument("--scenario", action="append", choices=["background", "window", "processes"])
    args = parser.parse_args()
    binary = str(Path(args.binary).resolve())
    scenarios = {
        "background": (["--background"], {}),
        "window": ([], {}),
        "processes": ([], {"COMPUTY_START_PAGE": "processes"}),
    }
    selected = args.scenario or ["background", "window"]
    results = {}
    for scenario in selected:
        arguments, extra = scenarios[scenario]
        runs = []
        for index in range(args.runs):
            data = tempfile.mkdtemp(prefix="computy-benchmark-")
            if args.reports:
                shutil.copytree(args.reports, Path(data) / "reports")
            environment = os.environ | extra | {"COMPUTY_DATA_DIR": data}
            result = run(binary, arguments, args.settle, args.duration, args.interval, environment)
            print(f"{args.label} {scenario} #{index + 1}: {result['pss_mib']} MiB PSS, {result['cpu_percent']} % CPU, {result['threads']} threads", flush=True)
            runs.append(result)
            shutil.rmtree(data, ignore_errors=True)
            time.sleep(2)
        results[scenario] = summarize(runs)
    document = {
        "label": args.label,
        "binary": public_path(binary),
        "binary_mib": round(os.path.getsize(binary) / 1024 ** 2, 2),
        "package_mib": round(args.package.stat().st_size / 1024 ** 2, 2) if args.package else None,
        "date": time.strftime("%Y-%m-%d"),
        "machine": {
            "kernel": platform.release(),
            "cpus": os.cpu_count(),
            "gpus": gpu_names(),
            "session": os.environ.get("XDG_SESSION_TYPE", ""),
        },
        "method": {
            "runs": args.runs,
            "settle_seconds": args.settle,
            "sample_seconds": args.duration,
            "interval_seconds": args.interval,
            "memory": "median of samples; PSS summed over the whole process tree",
            "isolation": "private D-Bus session and temporary data directory per run",
        },
        "scenarios": results,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(document, indent=2) + "\n")
    print(f"→ {args.output}")


if __name__ == "__main__":
    main()
