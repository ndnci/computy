# Diagnostic methodology

Computy preserves observations, their sources, parameters and limitations. It never turns missing information into a successful result or attributes a memory error to a particular module without supporting evidence.

## Memory

The Rust engine reserves a bounded allocation, attempts `mlock` / `VirtualLock`, and reports whether locking succeeded. The requested size cannot exceed 2048 MiB, one third of available memory, or one quarter of visible memory. Excessive requests are rejected.

Each pass applies five pattern families: virtual addresses, zeros/ones, checkerboard, a deterministic pseudorandom sequence, and a moving bit based on the word index. Each pattern is filled, checked in ascending order while inverting, then checked in descending order while restoring. Volatile reads and writes prevent the compiler from eliminating accesses. Mismatches are counted; the first offset, expected value and observed value are preserved.

This is a userspace check. It does not remap all physical memory, disable caches, perform Rowhammer testing, or implement the complete Memtest86+ suite. Without locking, pages may be swapped out. The moving-bit pattern does not exercise all 64 positions of every word within a single pass.

Accessible Linux EDAC counters accumulate since boot. They are not presented as errors caused exclusively by the current diagnostic.

Sources: [Memtest86+ algorithms and limitations](https://github.com/memtest86plus/memtest86plus#memory-testing-philosophy), [intermittent failures and MemTest86 test duration](https://www.memtest86.com/tech_execution-time.html), [Linux kernel EDAC](https://www.kernel.org/doc/html/latest/driver-api/edac.html).

## Storage

`smartctl --scan --json` enumerates devices, then `--all --json --nocheck=standby,0` reads their telemetry without deliberately waking a standby drive. Calls have timeouts, use no shell, and are limited to 16 devices. Missing tools, denied access and unexposed devices remain unavailable.

The smartctl exit code is a **bitmask**. Health alerts, NVMe media errors, critical warnings, pending/uncorrectable ATA sectors, reallocated sectors and historical logs are interpreted separately. A transport failure must not hide an explicit hardware alert. Relevant raw values remain in the report.

The file check creates a unique file with `create_new`, writes deterministic 1 MiB blocks that differ by index, calls `sync_all`, closes and reopens the file, then compares every byte read back. Linux receives an advisory cache eviction request. The file is deleted on success, error or cancellation; cleanup failures are reported. A 256 MiB free-space reserve is maintained.

This check covers the **volume containing Computy's data**. It does not scan all sectors or all drives and is not a throughput benchmark. Timing includes pattern generation, synchronization and comparison. It does not run a long SMART firmware self-test or replace a manufacturer's diagnostic. No raw writes, formatting or access to personal files is required.

Source: [official smartctl manual](https://github.com/smartmontools/smartmontools/blob/master/smartmontools/smartctl.8.in).

## CPU and sensors

Up to four threads calculate SHA-256 for one million `a` characters and an integer sum from 0 to 9999. Each answer is compared with a fixed known value; completed calculations and mismatches are recorded. Duration is bounded to 1–60 seconds. This does not exercise all instructions, the GPU, or prolonged thermal stability.

Temperatures are point-in-time readings from sensors exposed by the operating system. Missing temperatures or critical thresholds are not estimated.

## Inventory and replacement

Linux uses sysinfo and SMBIOS through dmidecode when accessible. Windows reads memory CIM classes; macOS uses system_profiler. Maximum capacities are **firmware declarations** that should be confirmed with the manufacturer. Windows `MaxCapacity` and `MaxCapacityEx` are converted from KiB.

A module's rated and configured transfer rates do not establish the platform's maximum supported rate. That also depends on the CPU, motherboard and firmware. DDR transfer rates are shown in MT/s. A DIMM/SODIMM form factor suggests a replaceable module; physical access must be confirmed in the manual. Unknown form factors remain unknown. Apple Silicon uses integrated, non-replaceable memory.

Sources: [Win32_PhysicalMemory](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-physicalmemory), [Win32_PhysicalMemoryArray](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-physicalmemoryarray), [Apple Diagnostics](https://support.apple.com/102550).

## Scores, coverage and history

| Observation | Check score |
|---|---:|
| No fault observed by the completed check | 10 |
| Actionable warning: wear or history needs review | 5 |
| Mismatch or hardware alert detected | 0 |
| Unavailable, not run, or interrupted | None |

The summary uses the lowest score among usable checks. An anomaly cannot be diluted by averaging. Unavailable checks reduce coverage and produce an incomplete assessment. An interrupted diagnostic receives no positive overall score. **Coverage counts usable checks; it is not a percentage of physically certified RAM or sectors.**

Comparisons preserve snapshots and warn when machines, sources, parameters or conditions differ. Higher capacity or speed is not evidence of better health. Windows preboot reports have a separate source; unknown allocation sizes and durations are never invented.
