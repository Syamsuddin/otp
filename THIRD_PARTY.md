# Third-party components

Compiler, semantic model, assembly emission, runtime, editor logic, and debugger logic are OTP Rust source under GPL-3.0-only. Infrastructure crates retain their original licenses. The artwork in assets/branding was newly generated for this project, not copied from the Borland reference.

Direct dependencies: minifb (window/input), font8x8 (bitmap font), png (original logo decoding). All resolved crates, including dependencies used only on other targets, are supplied in vendor/. Many inventory entries are not linked into the Linux binary. Cargo.lock records the exact dependency graph. This file is an inventory, not a claim that all target-specific crates are used.

The Rust standard library/toolchain is not vendored here. Rust 1.85.1 source: https://github.com/rust-lang/rust/tree/1.85.1 . Its license notices are distributed with the installed Rust toolchain. C runtime, OS libraries, SDK, assembler and linker come from the host system.

| Crate | Version | Declared license |
|---|---|---|
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 |
| bitflags | 2.13.2 | MIT OR Apache-2.0 |
| bitflags | 1.3.2 | MIT/Apache-2.0 |
| bumpalo | 3.20.3 | MIT OR Apache-2.0 |
| cc | 1.8.0 | MIT OR Apache-2.0 |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 |
| console_error_panic_hook | 0.1.7 | Apache-2.0/MIT |
| crc32fast | 1.5.2 | MIT OR Apache-2.0 |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 |
| find-msvc-tools | 0.1.14 | MIT OR Apache-2.0 |
| flate2 | 1.1.10 | MIT OR Apache-2.0 |
| font8x8 | 0.3.1 | MIT |
| futures | 0.3.34 | MIT OR Apache-2.0 |
| futures-channel | 0.3.34 | MIT OR Apache-2.0 |
| futures-core | 0.3.34 | MIT OR Apache-2.0 |
| futures-executor | 0.3.34 | MIT OR Apache-2.0 |
| futures-io | 0.3.34 | MIT OR Apache-2.0 |
| futures-macro | 0.3.34 | MIT OR Apache-2.0 |
| futures-sink | 0.3.34 | MIT OR Apache-2.0 |
| futures-task | 0.3.34 | MIT OR Apache-2.0 |
| futures-util | 0.3.34 | MIT OR Apache-2.0 |
| instant | 0.1.13 | BSD-3-Clause |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| js-sys | 0.3.106 | MIT OR Apache-2.0 |
| lazy_static | 1.5.1 | MIT OR Apache-2.0 |
| libc | 0.2.190 | MIT OR Apache-2.0 |
| libredox | 0.1.25 | MIT |
| memchr | 2.8.3 | Unlicense OR MIT |
| minifb | 0.28.0 | MIT OR Apache-2.0 |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 |
| once_cell | 1.21.4 | MIT OR Apache-2.0 |
| orbclient | 0.3.55 | MIT |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT |
| pkg-config | 0.3.34 | MIT OR Apache-2.0 |
| plain | 0.2.3 | MIT/Apache-2.0 |
| png | 0.17.16 | MIT OR Apache-2.0 |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib |
| redox_syscall | 0.9.4 | MIT |
| rustversion | 1.0.23 | MIT OR Apache-2.0 |
| sdl2 | 0.38.0 | MIT |
| sdl2-sys | 0.38.0 | MIT AND Zlib |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 |
| shlex | 2.0.1 | MIT OR Apache-2.0 |
| simd-adler32 | 0.3.10 | MIT |
| slab | 0.4.12 | MIT |
| syn | 3.0.9 | MIT OR Apache-2.0 |
| tokio | 1.53.2 | MIT |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| version-compare | 0.1.1 | MIT |
| wasm-bindgen | 0.2.129 | MIT OR Apache-2.0 |
| wasm-bindgen-futures | 0.4.79 | MIT OR Apache-2.0 |
| wasm-bindgen-macro | 0.2.129 | MIT OR Apache-2.0 |
| wasm-bindgen-macro-support | 0.2.129 | MIT OR Apache-2.0 |
| wasm-bindgen-shared | 0.2.129 | MIT OR Apache-2.0 |
| web-sys | 0.3.106 | MIT OR Apache-2.0 |
| winapi | 0.3.9 | MIT/Apache-2.0 |
| winapi-i686-pc-windows-gnu | 0.4.0 | MIT/Apache-2.0 |
| winapi-x86_64-pc-windows-gnu | 0.4.0 | MIT/Apache-2.0 |
| x11-dl | 2.21.0 | MIT |
| zlib-rs | 0.6.8 | Zlib |
| zmij | 1.0.23 | MIT |

Preserved top-level license/notice files are in third-party-licenses/. Full upstream source and any further nested notices remain in vendor/.
