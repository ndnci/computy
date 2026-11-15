#!/usr/bin/env bash
# Build the Linux .deb for the native GPUI desktop application.
# Run inside the GPUI toolchain: scripts/gpui-docker.sh scripts/package-deb.sh
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' desktop/Cargo.toml | head -1)"
target="${CARGO_TARGET_DIR:-$root/desktop/target}"

make -C third_party/computy-memory-boot/build/x86_64 GIT=none -j"$(nproc)" mt86plus
cargo build --release --locked --manifest-path native/Cargo.toml --no-default-features \
  --features boot-helper --bin computy-boot-helper --target-dir native/target/boot-helper
cargo build --release --locked --manifest-path desktop/Cargo.toml

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
install -Dm755 "$target/release/computy" "$stage/usr/bin/computy"
install -Dm755 native/target/boot-helper/release/computy-boot-helper "$stage/usr/libexec/computy-boot-helper"
install -Dm644 third_party/computy-memory-boot/build/x86_64/mt86plus "$stage/boot/computy-memory.efi"
install -Dm755 packaging/41_computy_memory "$stage/etc/grub.d/41_computy_memory"
install -Dm644 packaging/org.ndnci.computy.boot.policy "$stage/usr/share/polkit-1/actions/org.ndnci.computy.boot.policy"
install -Dm644 packaging/Computy.desktop "$stage/usr/share/applications/Computy.desktop"
install -Dm644 packaging/icons/32x32.png "$stage/usr/share/icons/hicolor/32x32/apps/computy.png"
install -Dm644 packaging/icons/128x128.png "$stage/usr/share/icons/hicolor/128x128/apps/computy.png"
install -Dm644 packaging/icons/256x256.png "$stage/usr/share/icons/hicolor/256x256/apps/computy.png"
install -Dm644 LICENSE "$stage/usr/lib/Computy/LICENSE"
install -Dm644 NOTICE "$stage/usr/lib/Computy/NOTICE"
install -Dm644 third_party/computy-memory-boot/LICENSE "$stage/usr/share/doc/computy/memory-boot-LICENSE"
install -Dm644 desktop/assets/fonts/OFL-DMSans.txt "$stage/usr/share/doc/computy/font-DMSans-OFL.txt"
install -Dm644 desktop/assets/fonts/OFL-Sora.txt "$stage/usr/share/doc/computy/font-Sora-OFL.txt"
mkdir -p "$stage/usr/share/doc/computy"
tar --exclude='*.o' --exclude='*.d' --exclude=mt86plus --exclude='*.iso' --exclude='*.debug' \
  -czf "$stage/usr/share/doc/computy/memory-boot-sources.tar.gz" third_party/computy-memory-boot

# Shared-library dependencies from the binary; Vulkan is loaded at run time.
shlib="$(mktemp -d)"
mkdir -p "$shlib/debian"
printf 'Source: computy\n\nPackage: computy\nArchitecture: amd64\n' > "$shlib/debian/control"
depends="$(cd "$shlib" && dpkg-shlibdeps -O "$stage/usr/bin/computy" 2>/dev/null | sed -n 's/^shlibs:Depends=//p')"
rm -rf "$shlib"
size="$(du -sk --exclude=DEBIAN "$stage" | cut -f1)"
install -d "$stage/DEBIAN"
cat > "$stage/DEBIAN/control" <<CONTROL
Package: computy
Version: $version
Architecture: amd64
Maintainer: NDNCI
Installed-Size: $size
Depends: ${depends:+$depends, }libvulkan1, pkexec
Recommends: mesa-vulkan-drivers | vulkan-icd, grub2-common
Section: utils
Priority: optional
Homepage: https://www.ndnci.com
Description: Comprenez, testez et suivez votre matériel.
 Native hardware diagnostics, live monitoring and local history.
CONTROL
install -m755 packaging/postinst "$stage/DEBIAN/postinst"
install -m755 packaging/postrm "$stage/DEBIAN/postrm"
mkdir -p release
deb="release/Computy_${version}_amd64.deb"
dpkg-deb --root-owner-group --build "$stage" "$deb" >/dev/null
sha256sum "$deb" | sed "s#release/##" > "$deb.sha256"
echo "$deb"
