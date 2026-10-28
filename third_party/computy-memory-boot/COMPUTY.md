# Computy Memory Boot

This separately executed pre-OS component is integrated into the Computy package.
It is derived from Memtest86+ **v9.00** (2026-10-15), distributed under GPLv2.
Upstream: https://github.com/memtest86plus/memtest86plus/tree/v9.00

Computy modifications, 2026-10-28:
- application title and footer identify the integrated Computy/NDNCI workflow;
- a green/sage framebuffer palette and dark default match the desktop identity;
- existing test algorithms, hardware initialization, error handling and report
  generation remain unchanged. Their original copyright notices are retained.

`COMPUTY-UPSTREAM.json` records the downloaded archive SHA-256 and original test
source hashes. `tests/boot-source.test.mjs` verifies the algorithms are unchanged.

Build with `make -C build/x86_64 GIT=none -j2 mt86plus` using GCC, binutils and make.
The root `scripts/build-native-extras.mjs` builds this component and archives all
corresponding modified sources for distribution in the .deb at
`/usr/share/doc/computy/memory-boot-sources.tar.gz`, alongside its GPL license.
See LICENSE for the license, copyright and warranty terms.
