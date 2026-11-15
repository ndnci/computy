#!/usr/bin/env bash
# Bundle the native desktop binary as Computy.app inside a disk image.
# Usage: scripts/package-macos.sh <output.dmg>   (after cargo build --release)
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
output="$1"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/desktop/Cargo.toml" | head -1)"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
app="$stage/Computy.app/Contents"
install -d "$app/MacOS" "$app/Resources"
install -m755 "$root/desktop/target/release/computy" "$app/MacOS/computy"
install -m644 "$root/packaging/icons/icon.icns" "$app/Resources/computy.icns"
install -m644 "$root/LICENSE" "$root/NOTICE" "$app/Resources/"
cat > "$app/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Computy</string>
<key>CFBundleDisplayName</key><string>Computy</string>
<key>CFBundleIdentifier</key><string>com.ndnci.computy</string>
<key>CFBundleVersion</key><string>$version</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>CFBundleExecutable</key><string>computy</string>
<key>CFBundleIconFile</key><string>computy.icns</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>LSMinimumSystemVersion</key><string>11.0</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
# Ad-hoc signature for local testing; not Developer ID signing.
codesign --force --deep --sign - "$stage/Computy.app"
ln -s /Applications "$stage/Applications"
hdiutil create -volname Computy -srcfolder "$stage" -ov -format UDZO "$output" >/dev/null
