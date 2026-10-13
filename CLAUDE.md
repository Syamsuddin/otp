# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

ODIN Turbo Pascal (OTP) v2.0 — a Rust reimplementation of Borland Turbo Pascal 7.0 (DOS) under GPL-3.0-only. Current milestone: `2.0.0-dev.2`. It is **not** a complete TP7 clone; the subset actually supported is tracked in [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md), and `docs/RENCANA.md` is the long-term target (not a status log).

Non-goals to keep in mind while editing: this does **not** use Free Pascal, does **not** transpile Pascal to Rust, and does **not** execute via bytecode/AST interpreter — it emits native assembly linked against a Rust `staticlib` runtime using the host's C toolchain as assembler/linker.

Primary docs to consult before changing behavior: `docs/COMPATIBILITY.md` (supported subset + gap list), `docs/ARCHITECTURE.md` (compiler/runtime layout), `docs/RENCANA.md` (target spec, acceptance gates). The project ships with vendored dependencies in `vendor/`; cargo is configured to build offline against them.

## Build, test, run

Requires Rust **1.85.1** (pinned in `rust-toolchain.toml`) plus a host C compiler (`cc`/`clang`) and Python 3. Standard workflow from the project root:

```sh
cargo build --workspace --release --locked --offline
cargo test  --workspace --locked --offline
python3 tests/native_suite.py --release
python3 scripts/package.py --skip-build
```

- Omit `--release` for debug builds; `tests/native_suite.py` without `--release` targets `target/debug/otpc`.
- `scripts/package.py` only packages the current host (Linux/Windows/macOS), it does not cross-compile. Without `--skip-build` it reruns build + cargo test + the native suite.
- Rust unit tests: `cargo test -p <crate> <filter>` (e.g. `cargo test -p otp-core parser::`).
- Single black-box case: `tests/native_suite.py` has no CLI filter — edit the `CASES`/`BAD`/`ERRORS` lists at the top, or compile+run a single example directly: `./target/release/otpc run examples/HELLO.PAS`.
- IDE smoke screenshots (headless, no X needed): `otp --screenshot out.ppm [--about|--tile|--cascade|--files|--windows]`.
- `otpc doctor` prints host target, resolved runtime archive path, and whether the linker (`OTP_CC`, default `cc`/`clang`) is reachable — run it first when diagnosing build failures.

Env vars the toolchain honors: `OTP_CC` (linker binary name/path only, never a shell string), `OTP_RUNTIME` (path to the host's `libotp_rtl.a` / `otp_rtl.lib`), `OTP_PREFIX` (installer target dir), `MACOSX_DEPLOYMENT_TARGET` (defaults vary; planned minimum 11.0).

## Workspace layout

Five crates, one shared compiler pipeline — do not duplicate logic across CLI and IDE:

- `crates/otp-core` — the compiler. Files: `lexer.rs`, `parser.rs`, `ast.rs`, `native.rs` (per-target asm emitter: Linux SysV, Win64, Darwin x86_64, Darwin ARM64), `build.rs` (invokes the host linker, resolves runtime archive, writes build dir `<output>.otp-build-<pid>`).
- `crates/otp-types` — type descriptors and `Real48` storage; consumed by both compiler and runtime so layouts stay in sync.
- `crates/otp-rtl` — Rust runtime linked as `staticlib`+`rlib`. Provides typed value helpers, string/record/array operations, text file I/O, partial CRT via ANSI, and the debugger hooks. Produced archive: `libotp_rtl.a` (unix) / `otp_rtl.lib` (windows).
- `crates/otp-cli` — the `otpc` binary (`check` / `build` / `run` / `debug` / `emit-asm` / `doctor`).
- `crates/otp-ide` — the `otp` desktop editor. `main.rs` owns input/menus/text model, `desktop.rs` owns window Rects, z-order, hit-testing, drag/resize, scrollbars, file browser. Uses `minifb` (software framebuffer), `font8x8`, `png` for the ODIN logo. The headless screenshot path shares the same renderer the GUI uses.

Compiler/runtime contract to preserve when editing:

- Every Pascal procedure/function emits as its own native symbol (`otp_user_N`); nested routines keep lexical parents. There is **no** bytecode/AST interpreter — don't introduce one.
- Values live in `otp-rtl`'s typed representation, not DOS/TP memory layout. Short strings cap at 255 bytes, `Real48` is 6-byte storage with f64 intermediates, aggregates (array/record) are structurally compared and copied by value except through `var` parameters.
- Index/range checks on arrays and strings are **always** emitted; `{$R-}` is not a licence to skip them. `{$R+}`, `{$Q+}`, `{$D+}` and their `-` forms are the only accepted directives, must appear before the first token, and apply program-wide — local/per-block directives are intentionally rejected, not silently promoted.
- Only Linux x86_64 is proven to link + run end-to-end at this milestone. The other backends emit asm but have not been executed; treat changes affecting Win64/Darwin ABI as unverified until linked on that host.
- The IDE compile/run runs in a worker; child processes own stdin/stdout/stderr pipes; the debugger speaks a tiny line/depth/state protocol over an **ephemeral loopback TCP socket** — the compiler inserts `otp_line` hooks before statements. No telemetry, no non-loopback network use.
- CLI and runtime never pass commands through a shell — everything is `std::process::Command` with argv arrays. Preserve this when adding external tool invocations.

## Pascal subset and conformance

When adding or changing language features, update `docs/COMPATIBILITY.md` in the same change — it is the source of truth for what the subset covers vs. the TP7 gap. Add a new row to `tests/native_suite.py` (`CASES` for success, `BAD` for compile errors matched by substring, `ERRORS` for runtime error codes) so the native suite guards it. Error numbers follow TP conventions where implemented (200/201/215/106…); don't invent new ones to make a test pass.

Unimplemented builtins must be rejected as `unknown`/`unimplemented` — never shim with silent no-ops (the `uses Graph`/Turbo Vision case is explicit). Likewise, do not "green" the test suite by loosening a check in `native_suite.py`; fix the compiler.

## Packaging artifacts

`dist/` holds build outputs; the shipped Linux tarball is `dist/OTP-2.0.0-dev.2-linux-x64.tar.gz` plus `install-linux.sh` (per-user install to `~/.local/bin` by default, honours `OTP_PREFIX`). `packaging/install-windows.ps1` is the equivalent per-user PS installer. `SHA256SUMS.txt` and `BUILD-MANIFEST.json` describe the shipped package. macOS `.app`/DMG and Windows ZIP are generated by `scripts/package.py` on their respective hosts only — do not try to produce them cross-platform.
