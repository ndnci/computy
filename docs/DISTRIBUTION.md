# Releases, installation and stores

## GitHub downloads

`.github/workflows/checks.yml` builds and tests the project on Linux, Windows,
Intel macOS and Apple Silicon. Pushes to `main` and pull requests validate builds;
manual runs also produce downloadable workflow artifacts. The same workflow is reused for releases.

| Target | Installer | CLI asset suffix |
|---|---|---|
| Linux x86-64 | `.deb` | `linux-x86_64` |
| Windows x86-64 | portable `.zip` | `windows-x86_64` |
| Intel Mac | `.dmg` | `macos-x86_64` |
| Apple Silicon Mac | `.dmg` | `macos-aarch64` |

CLI downloads are `.tar.gz` archives containing the native executable, `LICENSE`
and `NOTICE`. Extract before running (`tar -xzf FILE.tar.gz`, also available on
current Windows versions). Each archive and installer has a SHA-256 entry.

Linux's `.deb` includes the boot helper and GPL preboot component with its sources.
It is assembled by `scripts/package-deb.sh` inside the Debian toolchain image
(`scripts/gpui-docker.sh`), which keeps the binary's glibc requirement low. Package
dependencies are computed with `dpkg-shlibdeps`; WebKitGTK and GTK are no longer
required, while `libvulkan1` is. Windows/macOS keep their platform-specific preboot
workflows.

The interface renders with GPUI on the GPU: Vulkan on Linux, DirectX 11 on
Windows and Metal on macOS. Machines or virtual machines without a hardware
Vulkan driver can use Mesa's software driver (`mesa-vulkan-drivers`, recommended
by the package).

To publish a release:

1. Update `package.json`/lockfile and the `native` and `desktop` Rust package
   versions/lockfiles to the same version (`scripts/check-release.mjs` checks them).
2. Add a dated section to root `CHANGELOG.md`; keep future changes in `Unreleased`.
3. Run the relevant checks and commit/push the version.
4. In GitHub **Releases → Draft a new release**, choose the matching tag, for
   example `v0.2.0`, and the intended commit. Copy the corresponding changelog
   section into the release notes, then publish the release.
5. **Release downloads** validates the version and changelog, runs all builds and
   attaches installers, native CLI binaries and per-platform SHA-256 manifests.
   Downloads appear after every build succeeds; inspect the workflow if they do
   not appear. A failed platform prevents partial publication by this workflow.

No developer Windows or Mac computer is required for those CI builds: GitHub runs
native builds on each operating system. CI runners still do not validate all
physical hardware or desktop behavior. A public release must target a commit that
contains these workflows; older tags do not retroactively gain new workflows.

This public repository uses standard hosted runners, whose compute is free for
public repositories. It does not select larger paid runners. Intermediate build
artifacts are uploaded only for explicit manual/release runs and retained for one
day; durable downloads live in GitHub Releases. Storage quotas and optional paid
services are separate from free runner compute. Store accounts and signing are
also separate. See [GitHub Actions billing](https://docs.github.com/en/billing/concepts/product-billing/github-actions).

The publishing job alone has `contents: write`; pull-request build jobs cannot
publish. Download URLs are versioned. Existing assets are not overwritten on
reruns: publish a new version for changed binaries, especially after submitting
an installer URL to a store. SHA-256 files detect download corruption but do not
replace publisher signatures. The header's hardware refresh is not an updater.

References: [GitHub hosted runner architectures](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

## Windows and Microsoft Store

The CI Windows runner compiles the Rust desktop application and publishes it as a
portable `.zip`. For a local Windows build, install Node and Rust 1.100+, then run
`cargo build --release --manifest-path desktop/Cargo.toml` in a Windows checkout.
Building a Linux `.deb` does not produce a Windows executable. The GPUI build for
Windows and macOS is compiled by CI but has not been physically tested yet.

There are two Store submission paths to evaluate:

- Submit a traditional EXE/MSI through Partner Center using an immutable,
  versioned HTTPS installer URL. Microsoft requires a signed installer and signed
  PE files, plus silent installation support. Updates to that URL/version are
  your responsibility.
- Prepare a Store-compatible MSIX package. This is a separate packaging effort;
  the current zip output is not an MSIX. Microsoft handles signing for the Store
  MSIX submission path and provides Store-managed update delivery.

Reserve the product in Partner Center, prepare the listing, screenshots, privacy
information and age rating, then submit for certification. Current GitHub EXEs
are unsigned development outputs; the workflow does not yet contain a Windows
signing certificate or publish to the Store; an installer still has to be added. Signing and physical Windows checks
must be completed before presenting these files as Store-ready releases.

References: [Microsoft distribution paths](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/choose-distribution-path),
[EXE/MSI package requirements](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msi/app-package-requirements).

## macOS and Mac App Store

`scripts/package-macos.sh` creates separate Intel and Apple Silicon DMGs with
ad-hoc signing for testing.
This is not a Developer ID signature or notarization. Direct public distribution
should use an Apple Developer ID certificate and Apple's notarization workflow.
Store submission instead requires the appropriate App Store signing/provisioning,
App Store Connect listing, App Sandbox entitlements and review.

Review license compatibility as well as packaging before store submission:
GPLv3 does not permit imposing additional downstream restrictions. Do not assume
that a store's default distribution terms automatically satisfy the project's
GPL obligations. This change grants no store-specific license exception.
See [GPLv3 section 10](https://www.gnu.org/licenses/gpl-3.0.html#section10).

A dedicated Store configuration needs a capability audit: process inspection,
external hardware probes, local MCP networking and self-uninstallation can be
restricted by sandboxing or store rules. Do not claim that the current desktop
build already passes Store review or has identical access inside a sandbox.
Configure signing secrets only for trusted release jobs; never expose them to
pull requests from forks. No Apple credentials are configured by these changes.

References: [Apple notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution).

## Linux stores

The current release workflow provides direct `.deb` downloads. Snap
and Flathub require their own manifests, accounts/listings and review. Their
confinement changes access to host hardware, processes, external tools and boot
configuration. Add those packages only after validating their actual supported
features and explaining unavailable measurements. A Debian download is not an
APT repository: `apt install ./file.deb` upgrades the package, but ordinary
`apt update` cannot discover new Computy releases without a configured repository.

## Uninstalling and erasing local data

In **Preferences → Installation and data**, actions are separate:

- **Uninstall Computy** delegates to APT with PolicyKit on an installed Debian
  package, or Finder's Trash on a macOS app bundle.
  Portable files, including the Windows zip, must be removed through the file manager. This action does not
  ask Computy to erase history first. OS/uninstaller prompts remain authoritative.
- **Erase all data** requires a separate dialog and checkbox. It removes saved
  reports, preferences, pending Windows imports and diagnostic scratch files,
  and reloads the interface. It keeps user-exported files
  and an empty technical lock file. It does not remove prepared USB files, clear
  firmware state or alter a scheduled boot. Cancel any scheduled preboot test in
  its own wizard before uninstalling.

Erasure refuses an active diagnostic, another process's diagnostic lock or a
symlinked managed path. Unknown files are preserved, even in a custom data
folder. The CLI equivalent is `computy-cli erase-data --yes`; close the desktop
application first so its in-memory preferences cannot be saved again. These
maintenance mutations are not available over MCP. Package removal and store
submission are never executed on the developer host during automated tests.
