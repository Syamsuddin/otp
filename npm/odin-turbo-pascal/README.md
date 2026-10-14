# ODIN Turbo Pascal

Compiler Pascal native (`otpc`) dan IDE ber-nuansa Turbo Pascal 7.0 (`otp`), ditulis dalam Rust. Program Pascal dikompilasi menjadi executable native, tanpa interpreter, tanpa bytecode, dan tanpa Free Pascal.

```sh
npm install -g odin-turbo-pascal
otpc doctor
otpc run HELLO.PAS
otp
```

## Platform

| Sistem | Paket biner (otomatis) |
|---|---|
| macOS Apple Silicon | `@odin-turbo-pascal/darwin-arm64` |
| macOS Intel | `@odin-turbo-pascal/darwin-x64` |
| Linux x64 (glibc) | `@odin-turbo-pascal/linux-x64` |
| Windows x64 | `@odin-turbo-pascal/win32-x64` |

npm hanya mengunduh paket biner yang cocok dengan mesin Anda. Tidak ada skrip `postinstall`.

## Prasyarat

`otpc` memakai linker C milik sistem untuk menghasilkan executable. Pasang dulu:

- **macOS**: Xcode Command Line Tools (`xcode-select --install`)
- **Linux**: `build-essential` (menyediakan `cc`); desktop IDE butuh X11/XWayland
- **Windows**: Visual Studio C++ Build Tools + LLVM `clang` di PATH; jalankan dari *x64 Native Tools Command Prompt*

`otpc doctor` memeriksa apakah runtime dan linker sudah siap.

## Status

Versi `2.0.0-dev` adalah milestone pengembangan: subset Pascal yang berfungsi, **belum** clone TP7 100%. Daftar fitur yang didukung ada di [matriks kompatibilitas](https://github.com/Syamsuddin/otp/blob/main/docs/COMPATIBILITY.md).

## Lisensi

GPL-3.0-only. Kode sumber: <https://github.com/Syamsuddin/otp>
