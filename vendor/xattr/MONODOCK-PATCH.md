# Compatibility patch

This is `xattr` 0.2.3 from crates.io, retained under its original MIT/Apache-2.0 licenses. GPUI 0.2.2 depends on this version through `gpui_http_client` and `zed-async-tar`.

The only source change is in `src/sys/mod.rs`: on Linux and Android, the missing-attribute error uses `libc::ENODATA`. The former `libc::ENOATTR` alias is absent in current libc. Other supported platforms keep `ENOATTR`. This preserves the old crate's API without downgrading libc or other application dependencies.

Remove the Cargo patch and this directory once an upstream GPUI release removes the dependency on xattr 0.2.3. Original source: <https://crates.io/crates/xattr/0.2.3>.
