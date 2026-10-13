# Catatan implementasi ODIN Turbo Pascal v2.0

Tanggal pengerjaan: 13 Oktober 2026. Label pengguna: v2.0. Identitas build: 2.0.0-dev.1.

## Pembaruan jendela dev.2

Snapshot aktif sekarang 2.0.0-dev.2. Perubahan dari dev.1: bingkai editor per dokumen, Tile/Cascade/Zoom, drag/resize, scrollbar, daftar dokumen, browser Open/Save, konfirmasi overwrite dan close/save. F5 menjadi Zoom, output pindah ke Ctrl+F5. Compiler dan runtime tidak diubah selain nomor versi package.

Build release Linux dan 48 tes native lulus kembali. Terdapat 11 tes editor (9 baru) dan 1 tes tipe, total 12. Pemeriksaan input jendela menggunakan handler headless yang sama dengan aplikasi. Renderer aktual diperiksa. Belum uji window manager fisik atau native Windows/macOS. Lihat WINDOWS-dev2.md dan ui-test-report.json. Catatan di bawah merupakan riwayat dev.1, bukan jumlah tes dev.2.

## Hasil konkret dev.1

Lima crate Rust telah dibuat dari awal: tipe/Real48, compiler core, runtime, CLI, dan IDE desktop. Compiler menghasilkan assembly sendiri, kemudian memakai assembler/linker sistem. Program Pascal yang diuji berjalan sebagai executable ELF native, termasuk rekursi, lexical scope bersarang, parameter value/var, array/record, string, real, file teks, dan debugger statement. Build source tidak bergantung pada compiler Pascal lain.

Enam contoh pembelajaran, artwork ODIN, GPLv3, source dependensi vendored, installer per-user Linux/Windows, generator paket `.app`/DMG pada Mac, serta workflow CI empat host telah ditambahkan. Editor memiliki open/save, banyak dokumen, selection, undo/redo, find/replace, compile/run, input/output, trace, step over, breakpoint, dan snapshot scalar.

## Bukti dan batas pengujian

- Host pengujian: Linux x86_64, Ubuntu 24.04, Rust 1.85.1.
- Build workspace debug dan release berhasil. Build/test dengan dependency vendored dalam mode offline berhasil.
- Tiga unit test Rust lulus: layout Real48/descriptor tipe, editing/undo/selection Unicode, dan pencarian dengan wrap.
- Suite black-box menjalankan compiler dan executable yang dihasilkannya. Seluruh 48 dari 48 kasus lulus. Hasil final per-kasus disertakan dalam `native-test-report.json`.
- Debugger diuji melalui koneksi loopback nyata: berhenti sebelum assignment, step ke baris berikutnya, memasang breakpoint, continue, lalu memeriksa nilai berubah dan stdout.
- `file`/`objdump` menunjukkan executable ELF x86_64 dengan `main` dan call runtime native. Ini bukti native pada Linux, bukan bukti kompatibilitas TP7 lengkap.
- Renderer editor dan About dijalankan headless dan diperiksa visual; `IDE-preview.png` berasal dari renderer yang sama. Interaksi desktop melalui display server fisik belum diuji dalam lingkungan ini.
- Assembly untuk empat target berhasil dikeluarkan. Windows/macOS belum dilink atau dieksekusi dalam sesi ini. CI disediakan untuk pengujian tersebut dan belum dijalankan.
- Installer Linux diuji pada prefix terpisah di workspace, lalu compiler yang terpasang dipakai untuk membangun dan menjalankan contoh.

## Gerbang yang belum terpenuhi

Milestone ini belum menuntaskan G1 maupun gerbang akhir dalam RENCANA. Hal yang masih terbuka mencakup unit/OOP/pointer dan sisa bahasa TP7, seluruh RTL/library, DOS/CRT lengkap, BGI, Turbo Vision, kesamaan UI/event/keybinding, differential test terhadap TP7 asli, native linker mandiri, serta paket terverifikasi pada Windows dan kedua arsitektur macOS. Tidak ada klaim 100% kompatibel, siap produksi, atau semua OS sudah siap install.

`docs/COMPATIBILITY.md` memisahkan setiap kelompok fitur tersedia dan belum tersedia. Untuk melanjutkan, gunakan source ini sebagai baseline, jalankan ulang suite, lalu tambah implementasi beserta kasus pembanding TP7 sebelum mengubah status kompatibilitas.

## Keputusan implementasi awal

- Infrastruktur window/framebuffer memakai minifb dan font8x8 agar desktop native bisa berjalan pada tiga keluarga OS. Rencana awal winit/softbuffer adalah kandidat, bukan persyaratan wajib.
- Backend native awal memakai helper runtime bertipe. Ini menyediakan pipeline executable sendiri tanpa menunggu optimisasi machine-level dan layout memori DOS selesai.
- Real48 storage tersedia, tetapi aritmetika software exact TP7 masih merupakan pekerjaan terpisah.
- Direktif lokal yang belum bisa ditiru ditolak eksplisit. Tidak dibuat stub unit kosong yang menyembunyikan ketidaktersediaan fitur.
- Debugger memakai hook statement pada binary native, dengan kanal loopback. Dukungan debugger TP7 lanjutan belum diwakili hook ini.
- Paket saat ini memerlukan native linker yang terpasang. Persyaratan akhir tanpa toolchain eksternal belum terpenuhi.

## Rujukan engineering tambahan

Rujukan TP7, Borland manuals, distribusi, lisensi, dan format executable berada pada RENCANA bagian sumber. Tambahan untuk implementasi ini:

- Rust install/toolchain: https://doc.rust-lang.org/book/ch01-01-installation.html
- minifb milik pengembang: https://github.com/emoon/rust_minifb
- API minifb: https://docs.rs/minifb/0.28.0/minifb/
- GitHub runner matrix: https://docs.github.com/en/actions/reference/runners/github-hosted-runners
- Image Windows build tools: https://github.com/actions/runner-images/blob/main/images/windows/Windows2022-Readme.md

Runner CI menggunakan `macos-15-intel` dan `macos-15` untuk memisahkan Intel dan ARM64 sesuai daftar runner saat diperiksa. Tersedianya runner bukan bukti build OTP sudah berhasil di sana.
