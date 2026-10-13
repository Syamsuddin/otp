# Perbaikan jendela OTP v2.0-dev.2

Versi: 2.0.0-dev.2. Tanggal: 13 Oktober 2026.
Acuan interaksi: editor OTP v1.0 Python/Tk. Implementasi baru tetap Rust/minifb.

## Fitur yang ditambahkan

| Kontrol | Cara menggunakan |
|---|---|
| Jendela per dokumen | Setiap dokumen memiliki judul, nomor, tanda `*` bila berubah, posisi, ukuran dan viewport sendiri. Klik jendela untuk mengaktifkan. |
| Tile | Window > Tile menyusun dokumen berdampingan. Maksimal 12 dokumen per kelompok agar teks tetap terbaca. Untuk kelompok lain pilih dokumen lewat F6/List lalu Tile. |
| Cascade | Window > Cascade menumpuk bingkai dengan judul yang bergeser. |
| Zoom / Restore | F5 atau tombol `[+]` / `[-]` memperbesar editor ke area kerja editor, lalu mengembalikan ukuran sebelumnya. |
| Move / Resize | Seret batang judul. Seret tanda `/` di sudut kanan bawah untuk mengubah ukuran. Batas minimum 24 x 6 sel, jendela tetap berada dalam area editor. |
| Close | Alt+F3, Window > Close, atau `[X]`. Dokumen berubah meminta SAVE atau DISCARD kemudian Enter. Esc membatalkan. Setelah semua ditutup, tersedia satu dokumen kosong. |
| Next / Previous | F6 dan Shift+F6. Window > List menampilkan nama/path semua dokumen. Pilih dengan panah/Enter atau klik. |
| Scrollbar | Panah dan drag pada scrollbar vertikal/horizontal. Roda mouse menggulir vertikal. Shift+roda mouse menggulir horizontal. |
| Output | Ctrl+F5 memperbesar/mengembalikan panel output. F5 sekarang dipakai Zoom mengikuti OTP v1.0. |

## Membuka berkas

1. Tekan F3.
2. Dialog menampilkan folder aktif, subfolder, dan berkas Pascal. Filter awal `.pas`, `.pp`, `.inc`, tanpa membedakan kapital.
3. Klik berkas untuk memilih. Klik dua kali atau Enter untuk membuka pilihan. Folder ditandai `[DIR]`.
4. Tab memindahkan fokus antara daftar dan kolom Name. Ketik nama relatif atau path lengkap pada Name. Delete mengosongkan kolom.
5. Ctrl+U atau tombol Up menuju induk folder. Ctrl+A atau All/Pascal mengganti filter. Tombol Open menerima pilihan, Cancel/Esc membatalkan.
6. Kegagalan membuka berkas tetap menampilkan dialog dengan pesan. Membuka path yang sudah aktif tidak menduplikasi dokumen.

Pada Windows, path lengkap seperti `D:\Proyek\HELLO.PAS` juga dapat diketik untuk pindah drive. Belum ada daftar drive khusus.

## Menyimpan dan menutup

1. F2 menyimpan dokumen yang telah memiliki path. Dokumen baru membuka browser Save.
2. File > Save as membuka browser untuk memilih nama/lokasi baru. Nama tanpa ekstensi diberi `.pas`.
3. Bila berkas tujuan telah ada, dialog menampilkan path dan meminta `YES` lalu Enter sebelum menimpa. Esc membatalkan.
4. Save As ke berkas yang terbuka pada editor lain ditolak agar perubahan kedua dokumen tidak saling menimpa.
5. Alt+F3 pada dokumen berubah meminta `SAVE`, `DISCARD`, atau Esc. Gagal menyimpan tidak menutup dokumen atau menghapus tanda perubahan.
6. Saat program berjalan, berpindah source, menutup source, dan Save As diblokir. Hentikan dengan Ctrl+F2 terlebih dahulu.

## Build dan menjalankan

```sh
cargo build --workspace --release --locked --offline
cargo test --workspace --locked --offline
python3 tests/native_suite.py --release
./target/release/otp
```

Untuk Mac Intel, jalankan build tersebut pada Mac dengan Rust 1.85.1 x86_64 dan Xcode Command Line Tools. Mac Apple Silicon memakai Rust aarch64. Windows memakai toolchain MSVC dan lingkungan C++ sesuai README. Perbaikan jendela tidak memerlukan dependency tambahan.

## Bukti pemeriksaan

- Build release Linux berhasil.
- 11 tes editor lulus, termasuk 9 tes jendela/file browser baru. Bersama 1 tes tipe/runtime layout: 12 tes Rust lulus.
- 48 dari 48 kasus compiler/native lulus kembali pada Linux.
- Handler keyboard dan mouse diuji headless: Tile/Cascade/Zoom, fokus dan drag, resize, scroll, pemetaan klik setelah scroll, file browser, cancel/overwrite, close/save, dan penjagaan source saat proses berjalan.
- Renderer aktual diperiksa untuk editor, Tile, Cascade, browser berkas, daftar jendela, dan About. Gambar ada dalam folder docs.
- Belum dilakukan pengujian input melalui window manager fisik atau build native Windows/macOS dalam sesi ini. Tes headless tidak menggantikan pengujian perangkat tersebut.

## Batas implementasi

Panel output tetap menyatu dengan IDE. Jendela console terpisah seperti OTP v1.0 belum dibuat. Ukuran grid desktop tetap 100 x 35 sel. Area jendela editor 100 x 22 sel, sisanya panel output dan status. Tile dibatasi per kelompok 12 dokumen. Font bitmap belum mendukung tampilan semua karakter Unicode walaupun nama/path diproses sebagai string host. Clipboard OS, penataan jendela persisten, dan peniruan pixel/event TP7 penuh belum tersedia.

Buku PDF edisi 1.0 mendokumentasikan dev.1. Untuk perubahan shortcut dan jendela dev.2, gunakan panduan ini.
