# Arsitektur implementasi dev.1

`otpc` dan `otp` berbagi `otp-core`. Satu jalur compiler dipakai oleh CLI, IDE, dan pengujian.

Compiler membaca source bytes, memproses direktif dan token, lalu membuat AST. Pemeriksaan scope/tipe mendahului pembentukan assembly. Assembler dan linker host menggabungkan assembly tersebut dengan static library `otp-rtl` menjadi executable native.

`otp-types` mengodekan tipe dalam descriptor yang dibaca `otp-rtl` saat frame diinisialisasi. Integer TP yang didukung tetap memiliki lebar 8/16/32, walaupun pointer dan host ABI 64-bit. Record/array terkelola dan short string bukan layout alamat DOS. Real48 memiliki enam byte storage, aritmetika intermediate memakai f64.

Setiap procedure/function menjadi simbol assembly sendiri (`otp_user_N`). IF/loop/CASE menjadi instruksi branch; call dan return memakai ABI host. Generator mempertahankan lexical parent untuk routine bersarang. Helper runtime membawa operand typed dan referensi variabel, tanpa dispatcher instruksi bytecode atau loop evaluasi AST. Parameter value aggregate disalin, parameter `var` menggunakan cell yang sama. Build mempertahankan executable sebelumnya jika parsing atau linking gagal.

Backend x86_64 Linux memakai SysV, Windows memakai empat register argumen plus shadow space, macOS x86_64 memakai prefix simbol Darwin. Backend Apple ARM64 menggunakan register x0-x4, frame x29/x30 dan literal page relocations. Target yang benar-benar telah diuji eksekusi pada pengerjaan ini hanya Linux x86_64. Emitter lintas target tidak membuktikan keberhasilan linking pada OS lain.

IDE memakai `minifb` untuk window/input dan framebuffer software. `font8x8` adalah font sumber terbuka, bukan font yang disalin dari Borland. `png` membaca artwork asli ODIN yang disertakan. Editor model terpisah dari renderer; fungsi headless `otp --screenshot file.ppm [--about|--tile|--cascade|--files|--windows]` menggunakan renderer yang sama.

Compile/run berjalan pada worker agar event loop desktop tetap responsif. Child process memiliki pipe stdin/stdout/stderr. Compiler dapat menyisipkan `otp_line` sebelum statement. Debugger memasang listener loopback ephemeral, program native mengirim nomor baris/depth/state, dan frontend mengirim STEP/NEXT/CONT/BREAK/QUIT. Penutupan aplikasi meminta child dihentikan dan di-reap. Ini debugger source minimum, bukan debugger mesin lengkap.

CLI dan runtime menghindari shell untuk compile/run: perintah dikirim sebagai argumen `std::process::Command`. Paths tidak digabung menjadi shell command. `OTP_CC` hanya nama/path executable, bukan string perintah shell. `OTP_RUNTIME` harus menunjuk arsip library sesuai host. Tidak ada telemetry atau koneksi jaringan selain debugging pada loopback.

## Jendela dev.2

`otp-ide/src/desktop.rs` mengatur Rect setiap Document, urutan lapisan jendela, hit-testing, drag, scrollbar, daftar dokumen, serta browser berkas. `main.rs` tetap mengelola input host, event compiler/runtime, menu, dan model teks. Test editor menjalankan handler keyboard/mouse yang sama dengan loop aplikasi. Viewport manual tidak dipaksa kembali ke kursor oleh setiap frame render. Perpindahan atau penutupan source diblokir selama program berjalan.
