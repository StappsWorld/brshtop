#!/bin/sh

# This script calls `bindgen` (https://crates.io/crates/bindgen)
# in order to create Rust bindings for XNU libproc-related things.
#
# `bindgen` must be installed and available in `$PATH`.
#
# Caller also need to download macOS SDK and change `-I` flag below
# in order to generate bindings.
# Do note that macOS 10.9 (Mavericks) SDK **MUST** be used for code generation.

bindgen \
  --rust-target 1.36 \
  --whitelist-function "proc_.*" \
  --whitelist-var "proc_.*" \
  --whitelist-var "PROC_.*" \
  --whitelist-type "proc_.*" \
  --whitelist-type "rusage_.*" \
  --whitelist-var "RUSAGE_.*" \
  wrapper.h \
  -- \
  -IMacOSX-SDKs/MacOSX10.13.sdk/usr/include \
  > src/generated.rs
