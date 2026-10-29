# Cleanup, applications and cooling

## Mandatory cleanup preview

Computy 0.2 adds a Rust cleanup engine shared by the desktop and CLI. Nothing is
removed by scanning. The desktop opens a second confirmation with the selection
and file-size total. Execution requires a persisted preview ID and explicit
confirmation. Editing the policy invalidates the desktop preview. A plan expires
after 24 hours and is consumed before deletion; it cannot be replayed.

Default sources are the current user's cache on Linux, `~/Library/Caches` on
macOS, and `%LOCALAPPDATA%/Temp` on Windows, when available without symbolic
links. The default condition selects files last modified more than seven days
ago. Arbitrary personal folders are never added automatically. Add specific
absolute directories yourself, including project roots for the optional
`node_modules`-only scan. Source code outside those dependency directories is
not selected by that mode. Node dependencies can be reinstalled with the
project's package manager, but locally modified files inside `node_modules`
would be lost: review the preview first.

Policies support nested `all` (AND), `any` (OR), and `not`, exact file names,
extensions, bounded Rust regular expressions on paths, minimum/maximum bytes,
content-detected MIME types, modification/creation timestamps, and minimum age.
The editor's dates are local time; JSON timestamps are Unix milliseconds.
Unknown creation dates remain unknown through negation. Content detection reads
at most 8 KiB; unrecognized UTF-8 content becomes `text/plain`, otherwise
`application/octet-stream`. This is a filter, not authoritative file-format
validation. Excluded paths override all inclusion/filter decisions. Filesystem
creation time is not a universal "date added" to a folder.

The scan skips links, nonregular files, `.git`, `.ssh`, `.gnupg`, Computy's own
data and files owned by another UID on Unix. It never requests administrator
privileges for filesystem cleanup. System/application directories and broad
home/root directories are rejected. There are limits of 16 roots, 64 exclusions,
64 filter conditions, eight nesting levels, a 30-second filesystem scan, 150,000 inspected entries and 30,000
selected files. An incomplete scan cannot authorize an automation. Narrow it
instead. Permissions failures and unavailable Docker adapters appear in the
preview. Empty directories are deliberately retained: no recursive directory
removal is used.

Before deletion, file identity, size and modification metadata are checked.
Unix deletion opens each parent with `O_NOFOLLOW` and uses a directory handle,
so replacing an ancestor with a symlink cannot redirect cleanup. Concurrent
writers are still possible; pause programs whose caches you are cleaning.
This is not a defense against a malicious process running as your own user.
Windows uses metadata revalidation and rejects symbolic-link paths. Cleanup,
diagnostics and Computy's own data erasure share an exclusive operation lock.

The displayed total is the logical size of selected files. Hard links,
compression, sparse files, snapshots and open file handles can make the change
in available disk space smaller. Execution reports deleted logical bytes,
skipped items and per-item failures; it never claims that every byte has become
immediately available on disk. Deletion is permanent, not a move to Trash.

## Docker

The adapter detects a local Docker endpoint and enumerates unused images,
stopped containers and optionally unreferenced volumes. It rejects TCP/SSH
contexts. Docker items start unchecked. A stopped container can contain valuable
writable-layer data; a volume may contain a database even when no container
references it. Select these resources deliberately.

Objects are checked again immediately before removal. Computy invokes exact
`docker image rm`, `docker container rm` or `docker volume rm` commands without
force; it never substitutes an unbounded `system prune` for an approved preview.
Images referenced by any container are excluded. No Docker object is automated.
Image sizes include shared layers and are shown separately as estimates; volume
sizes may be unknown. Build-cache/network pruning is not offered because this
adapter cannot bind it to an exact, inspectable per-object deletion plan.

Docker inspection can be expensive, so it runs only on request and has bounded
subprocess timeouts. See the upstream [pruning documentation](https://docs.docker.com/engine/manage-resources/pruning/)
and [`system df` documentation](https://docs.docker.com/reference/cli/docker/system/df/).

## Automations

Create a schedule only from a complete, fresh file preview. Confirmation grants
permission for **future files matching the policy**, not only the IDs currently
checked. To exclude something permanently, add an excluded path or a NOT rule.
Every scheduled run creates a new dry run; it removes nothing when the logical
file total exceeds the approved byte limit. Schedules have an interval of
1–8,760 hours, a per-run limit, a next-run time and a retained last result.

The desktop checks due schedules every minute while running, including in the
background. Sleeping, logging out or fully quitting stops execution. Missed
runs are processed after a later launch; the next due date starts from that run,
without executing a backlog repeatedly. Login monitoring is enabled by default
and can be turned off. Keep indicators enabled for an unattended background
session. CLI-only users can call `clean run-due` from their own OS scheduler;
Computy does not install a privileged service or wake the computer.

```sh
computy-cli clean sources
computy-cli clean preview --policy cleanup.json
computy-cli clean execute PREVIEW_ID --yes
computy-cli clean schedule PREVIEW_ID --hours 168 --max-bytes 1073741824 --yes
computy-cli clean schedules
computy-cli clean run-due
computy-cli clean unschedule SCHEDULE_ID
```

Create a schedule before consuming its preview, or scan again. A sample policy:

```json
{
  "roots": ["/home/you/.cache"],
  "exclude": ["/home/you/.cache/keep"],
  "rule": {
    "op": "all",
    "rules": [
      {"op": "older_days", "value": 14},
      {"op": "any", "rules": [
        {"op": "extension", "value": "log"},
        {"op": "mime", "value": "application/zip"}
      ]},
      {"op": "not", "rule": {"op": "name", "value": "important.log"}}
    ]
  },
  "node_modules": false,
  "docker": false,
  "docker_volumes": false
}
```

## Application inventory

The searchable desktop view lists launchable desktop entries, Debian packages,
Flatpak applications and Snaps on Linux; application bundles in standard folders
on macOS; and uninstall-registry/Store packages on Windows. Linux can include
system packages with an explicit checkbox. Each adapter reports the manager,
package identity, version and known installation/launcher paths. Portable
software outside these registries/folders is not discoverable universally.
Opening a folder does not assert that all of the application's files live there.

Launch uses the registered desktop launcher, Flatpak/Snap, macOS `open`, or an
unambiguous Windows Start-menu identity. An unavailable launcher disables the
button. Uninstallation resolves the inventory ID again, previews the manager's
removal action, and requires confirmation. Debian removal shows the `apt-get`
simulation including dependencies and uses PolicyKit; Flatpak/Snap use their
managers; Windows uses an exact interactive WinGet removal; macOS asks Finder
to move the application bundle to Trash. Store packages without a verified
removal adapter remain explicitly unsupported. OS authorization may be needed.

Only Flatpak/Snap currently expose the optional package-managed data removal.
Other adapters preserve data or defer removal; Computy does not guess folders
from an app's display name. Even package-managed deletion does not remove
personal documents saved elsewhere. Read the application's own backup/export
instructions before uninstalling it.

```sh
computy-cli applications
computy-cli applications --id 'deb:PACKAGE' --action preview_uninstall
computy-cli applications --id 'deb:PACKAGE' --action launch
computy-cli applications --id 'deb:PACKAGE' --action reveal
computy-cli applications --id 'deb:PACKAGE' --action uninstall --yes
```

MCP exposes `computy_applications` and `computy_fans` as read-only tools. Cleanup,
scheduling, launching, and uninstallation are not available to a remote agent.
Local CLI JSON RPC and the main desktop window use the validated native methods.
The tray popover retains only monitor/preferences access.

## Fans

The cooling page and `computy-cli fans` read available Linux hwmon fan RPM,
PWM duty cycle and control-mode attributes. Windows queries fan inventory via
WMI but does not present its `DesiredSpeed` as a measured RPM. Unsupported
platforms/firmware return unavailable readings, never fabricated values.

Manual PWM, timed boost and custom fan curves are **not enabled** in 0.2. There
is no universal safe protocol. Each driver needs a validated adapter, privilege
boundary, thermal checks, fail-safe minimum and an independent watchdog that
restores automatic control if the UI exits or crashes. A generic slider would
not meet that requirement. The existing firmware controller stays in charge.
See the kernel's [hwmon interface](https://github.com/torvalds/linux/blob/master/Documentation/ABI/testing/sysfs-class-hwmon)
and [NCT6775 driver documentation](https://www.kernel.org/doc/html/latest/hwmon/nct6775.html).
