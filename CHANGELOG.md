# Changelog

Semua perubahan penting pada **ODIN Turbo Pascal (OTP)** v2.0 dicatat di sini.
Format mengikuti [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
versioning mengikuti [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
dengan suffix `-dev.N` untuk milestone pengembangan di jalur `2.0.0`.

## [Unreleased]

### Fixed
- **CI Windows**: `.gitattributes` (`* -text`) mematikan konversi CRLF saat checkout, yang merusak checksum `vendor/*/.cargo-checksum.json` sehingga `cargo build --offline` gagal di runner Windows.
- **CI Windows**: workflow memanggil `python` (bukan `python3` yang tidak tersedia di Windows) dan menyiapkan lingkungan MSVC (`ilammy/msvc-dev-cmd`) agar `clang` dapat menautkan program Pascal.
- **CI Windows**: rustc dikunci ke `link.exe` MSVC lewat path absolut (`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`), karena di Git Bash `link.exe` di PATH adalah utilitas coreutils milik Git.
- **CI macOS Intel**: runner `macos-13` yang sudah pensiun diganti `macos-15-intel`.

### Changed
- Workflow CI menampilkan info toolchain dan potongan log langkah yang gagal sebagai anotasi publik.

### Planned
- Universal macOS binary (x86_64 + arm64) di satu `.app`.
- CI multi-host hijau pada semua target (`linux-x64`, `macos-x64`, `macos-arm64`, `windows-x64`).
- Developer ID signing + notarization untuk DMG rilis.
- Fase RENCANA berikutnya: unit compiler + namespace, model ordinal/typed constants, API System dan file I/O lengkap.

## [2.0.0-dev.2] — 2026-10-14

Milestone pengembangan kedua: packaging multi-platform, build macOS Intel terverifikasi end-to-end, installer macOS per-user, dan README standar GitHub.

### Added
- **macOS Intel (x86_64) release** — tarball + `ODIN Turbo Pascal.app` (ad-hoc codesigned) + DMG (UDZO), dibangun pada macOS 15.7.9 Intel dengan 48/48 native tests lulus.
- **`packaging/install-macos.sh`** — installer per-user untuk macOS:
  - Hormati `OTP_PREFIX` (default `~/.local`) untuk CLI dan `OTP_APP_PREFIX` (default `~/Applications`) untuk `.app`.
  - Validasi arsitektur paket via `BUILD.json`: reject `macos-arm64` di host Intel; peringatkan Rosetta 2 untuk `macos-x64` di Apple Silicon.
  - Wrapper shell di `$prefix/bin/{otp,otpc}` yang export `OTP_RUNTIME` lalu `exec` biner asli (bukan symlink, karena `std::env::current_exe()` di macOS tidak resolve symlink — menyebabkan resolver runtime di [`crates/otp-core/src/build.rs`](crates/otp-core/src/build.rs) meleset).
  - Strip quarantine xattr dari `.app` yang terpasang (lewatkan Gatekeeper untuk ad-hoc signed dev build).
  - Re-sign ad-hoc kalau tanda tangan rusak saat copy.
  - Peringatkan jika `$prefix/bin` belum di PATH dan jika Xcode Command Line Tools belum terpasang.
- **README.md standar GitHub** — logo terpusat 240 px, 10 badge (version/license/rust/platform/tests/subset status), tabel kompat per platform, screenshot IDE 3×2, kutipan contoh program, tabel keybinding, bagian Berkontribusi dengan aturan "no silent shim" dan "jangan hijaukan test dengan melonggarkan check".
- **CHANGELOG.md** — jurnal rilis dengan format Keep a Changelog.
- **`.github/workflows/native.yml`** — CI matrix 4 host (ubuntu-24.04, macos-13 Intel, macos-14 ARM64, windows-2022): `cargo build --release --locked --offline` → `cargo test` → `python3 tests/native_suite.py --release` → `python3 scripts/package.py --skip-build` → upload artefak host.
- **`.gitignore`** — exclude `target/`, Pascal executable hasil kompilasi di `examples/`, `*.otp-build-*/`, `.DS_Store`, IDE caches, `__pycache__`.

### Changed
- **`scripts/package.py`** — tambahkan `install-macos.sh` ke daftar yang di-stage saat packaging; artefak macOS kini membawa ketiga installer.
- **`BUILD-MANIFEST.json`** — `verified_host` sekarang `"linux-x64; macos-x64 (Intel) build+native suite 48/48 on macOS 15.7.9"`; `other_hosts` dipertegas (`windows-x64` dan `macos-arm64` belum dieksekusi).

### Verified on this milestone
- Linux x86_64: build + 48/48 native tests + 12 Rust tests + 11 editor tests.
- macOS x86_64 (Intel, 15.7.9): build + 48/48 native tests + 12 Rust tests + 11 editor tests + DMG mount test + installer dry-run end-to-end (`otpc doctor` finds runtime, `otpc run HELLO.PAS` kompilasi + jalankan).
- macOS ARM64, Windows x64: source + build workflow + emitter assembly, **belum dieksekusi** di hardware tersebut.

### Known gaps vs. TP7
Lihat [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md) untuk daftar lengkap: pointer, Object Pascal, BGI, Turbo Vision, typed constants, TPU/DCU, ReadKey/KeyPressed, dan lainnya **belum** diimplementasikan dan ditolak secara eksplisit (bukan silent no-op).

## [2.0.0-dev.1] — 2026-10-14

Milestone pengembangan awal. Compiler + runtime + IDE pertama yang bisa dibangun dan menghasilkan executable native.

### Added
- **Compiler `otpc`** dengan jalur lexer → parser → semantic → native assembly emitter → linker driver. Backend assembly untuk Linux x86_64 (SysV), Windows x64, macOS x86_64 (Darwin), dan macOS ARM64. Hanya Linux x86_64 yang telah dilink + dijalankan end-to-end.
- **Runtime Rust `otp-rtl`** sebagai `staticlib` + `rlib`, menyediakan tipe terkelola, string/record/array operations, text file I/O, CRT parsial via ANSI, dan hook debugger.
- **IDE desktop `otp`** berbasis framebuffer `minifb` + `font8x8`:
  - Jendela per dokumen, Tile/Cascade/Zoom (F5), move/resize, scrollbar, roda mouse (plus Shift+roda horizontal), daftar jendela.
  - File browser dengan konfirmasi overwrite, filter Ctrl+A, Ctrl+U naik folder.
  - Find/Replace (`cari|pengganti`), undo/redo, selection, clipboard internal.
  - Compile Alt+F9, Run Ctrl+F9, Stop Ctrl+F2, Trace F7, Step over F8, Breakpoint Ctrl+F8.
  - Auto-recovery: jendela yang ditutup paksa menyimpan draft ke `odin-turbo-pascal-recovery` di temp OS.
  - Headless screenshot: `otp --screenshot out.ppm [--about|--tile|--cascade|--files|--windows]`.
- **Debugger loopback** — hook `otp_line` sebelum setiap statement; frontend/child berkomunikasi via TCP loopback ephemeral; CLI debugger menerima `s`/`n`/`c`/`b NOMOR`/`q`.
- **Subset Pascal** — Byte/ShortInt/Integer(16-bit)/Word/LongInt(32-bit), Real48 (6 byte, f64 intermediate), Boolean/Char, short string sampai 255 byte, array multidimensi dengan **range-check selalu aktif**, record, parameter value/var, procedure/function bersarang + rekursi, IF/ELSE/WHILE/REPEAT/FOR to/downto/CASE rentang/EXIT/Break/Continue.
- **CRT parsial via ANSI** — ClrScr, ClrEol, GotoXY, WhereX/Y, TextColor, TextBackground.
- **System builtin** — Abs, Sqr, Sqrt, Sin, Cos, ArcTan, Ln, Exp, Round, Trunc, Pi, Odd, Inc, Dec, Succ, Pred, Lo, Hi, ParamCount, ParamStr, Halt, Delay, Random, Randomize, IOResult.
- **Text file I/O** — Assign, Reset, Rewrite, Close, Erase, Flush, Eof.
- **Direktif program-wide** — `{$R+/-}`, `{$Q+/-}`, `{$D+/-}` wajib sebelum `program`. Direktif lokal/per-block **ditolak**, bukan silent-promoted.
- **Enam contoh Pascal** — HELLO.PAS, BELAJAR.PAS, FAKTORIAL.PAS, PARAMETER.PAS, ARRAY_RECORD.PAS, FILE_TEKS.PAS.
- **48 native black-box tests** (`tests/native_suite.py`) + 12 Rust unit tests + 11 editor tests.
- **Pemaketan Linux x86_64** — tarball + `install-linux.sh` per-user + `.desktop` entry + icon 512×512.
- **Pemaketan Windows x64** — ZIP + `install-windows.ps1` per-user + Start Menu shortcut (installer tersedia, build belum dieksekusi di Windows host).
- **Pemaketan macOS** — `scripts/package.py` membangun `.app` + DMG di Darwin host (eksekusi lintas-OS dilakukan di milestone 2.0.0-dev.2).
- **Dependensi ter-vendored penuh** di `vendor/` — `cargo` dikonfigurasi offline-by-default via `.cargo/config.toml`.
- **BUILD-MANIFEST.json** — SHA256 untuk setiap source + artefak rilis.
- **SHA256SUMS.txt** — digest siap verifikasi downloader.
- **Dokumentasi** — [COMPATIBILITY.md](docs/COMPATIBILITY.md) (matriks subset), [ARCHITECTURE.md](docs/ARCHITECTURE.md), [RENCANA.md](docs/RENCANA.md), [IMPLEMENTASI.md](docs/IMPLEMENTASI.md), [WINDOWS-dev2.md](docs/WINDOWS-dev2.md).

### Not implemented (and explicitly rejected as `unknown`/`unimplemented`)
- Pointer, `New`/`Dispose`, `GetMem`/`FreeMem`, segment/offset, absolute, port/mem, interrupt — **tidak** ada emulasi memori DOS.
- Object Pascal (object, method, constructor/destructor, virtual, inheritance).
- Unit Graph / BGI (driver, mode, palet, drawing, CHR font).
- Turbo Vision (Objects, Views, Drivers, Dialogs, Menus, App).
- `uses Dos` tidak memberi API Dos — hanya nama profil parsial yang dikenali.

[Unreleased]: https://github.com/Syamsuddin/otp/compare/v2.0.0-dev.2...HEAD
[2.0.0-dev.2]: https://github.com/Syamsuddin/otp/releases/tag/v2.0.0-dev.2
[2.0.0-dev.1]: https://github.com/Syamsuddin/otp/releases/tag/v2.0.0-dev.1
