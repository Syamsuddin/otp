# Rencana ODIN TURBO PASCAL (OTP) dalam Rust

Versi 0.2, 13 Oktober 2026. Keputusan produk dikunci berdasarkan jawaban Pak Syams. Dokumen ini menggantikan rancangan v0.1. Status implementasi: spesifikasi dan identitas visual, belum compiler Rust lengkap atau installer.

Tujuan: membangun ulang compiler, pustaka, IDE, debugger, dan lingkungan eksekusi OTP dengan implementasi sendiri dalam Rust. menyediakan aplikasi siap pasang untuk Windows, Linux, macOS Intel, dan macOS Apple Silicon.

## 1. Keputusan produk yang ditetapkan

| ID | Pokok keputusan | Spesifikasi |
|---|---|---|
| D01 | Acuan | Borland Turbo Pascal 7.0 DOS. Acuan kerja: edisi Inggris 1992, bukan 7.01, Borland Pascal, atau Turbo Pascal for Windows. Verifikasi build melalui distribusi masih harus dilakukan. |
| D02 | Hasil compiler | Executable native Windows, Linux, macOS Intel, dan macOS Apple Silicon. Default Compile menghasilkan executable untuk OS/arsitektur tempat OTP berjalan. Cross-compilation bukan syarat rilis awal. |
| D03 | Kesamaan | Bahasa, API/perilaku library, dan UI mengikuti TP 7.0 DOS. Kesamaan byte executable Borland tidak menjadi sasaran karena format dan arsitektur target berubah. |
| D04 | Implementasi | Compiler, library, dan aplikasi ditulis dalam Rust. Crate Rust untuk jendela, input, dan integrasi OS diperbolehkan. |
| D05 | Merek | Nama persis: ODIN Turbo Pascal. Singkatan OTP. Logo baru menggabungkan persona Odin, petir, bahasa Pascal, dan pembelajaran coding. |
| D06 | Pembanding | Penelusuran dan pemilihan bahan pembanding dikerjakan oleh pengembang. Pengguna tidak diwajibkan mencari bahan terlebih dahulu. |
| D07 | Lisensi | GNU GPL versi 3. Identifier implementasi dipilih GPL-3.0-only, sesuai versi yang diminta. Tidak menambahkan izin versi berikutnya atau pengecualian linking secara otomatis. |

Acuan logo berbeda dari acuan fungsional. Screenshot pengguna bertuliskan Turbo Pascal for Windows Version 1.5. Screenshot dipakai untuk gaya bitmap hitam/kuning, tipografi putih, dan aksen magenta. Menu, bahasa, serta library tetap mengikuti TP 7.0 DOS.

Perilaku bahasa dan library terdokumentasi menjadi kontrak. Cacat pada compiler lama ditangani melalui kasus kompatibilitas yang terukur, bukan persyaratan mempertahankan setiap cacat tanpa batas. Keputusan ini tidak mengizinkan penghilangan API yang sulit diimplementasikan.

GPLv3 diterapkan pada source OTP, termasuk runtime buatan proyek. Dokumentasi lisensi harus membedakan penggunaan compiler dari penyertaan kode runtime ke executable. GPLv3 membahas hasil program berdasarkan isi hasil tersebut. Karena itu tidak dijanjikan bahwa semua executable pengguna otomatis bebas dari ketentuan GPL ketika membawa runtime OTP. Tidak ditambahkan runtime exception tanpa keputusan pemilik [S16].

## 2. Hasil riset dan batas bukti saat ini

Empat manual Borland 1992 ditemukan. Tiga manual Turbo Pascal berjumlah 817 halaman PDF. manual Turbo Vision 640 halaman. Bagian terkait arsitektur dan daftar isi telah ditinjau, beserta tabel dan halaman visual tertentu. Ini bukan pernyataan bahwa seluruh 1.457 halaman sudah diaudit baris demi baris.

Ringkasan fakta acuan:

- **User’s Guide [S1]:** IDE mencakup pengelolaan jendela, bantuan, editor, konfigurasi, ObjectBrowser, Tools/Messages, serta debugging. TURBO dan TPX mempunyai lingkungan memori berbeda. Beberapa jendela dapat berbagi dokumen yang sama. README, FILELIST.DOC, dan HELPME!.DOC penting untuk melengkapi manual.
- **Language Guide [S2]:** kompatibilitas mencakup tipe, unit, objek, parameter, layout memori, pemanggilan, assembler, dan runtime. Real berukuran enam byte. perhitungan dengan 80x87 mempunyai persoalan presisi tersendiri. Pustaka mencakup System, Dos, Crt, Printer, Overlay, Strings, WinDos, Graph, Turbo3, dan Graph3.
- **Programmer’s Reference [S3]:** rujukan API, direktif, compiler baris perintah, diagnostik, tombol editor, dan tabel karakter. Nilai default opsi merupakan bagian perilaku. misalnya pemeriksaan range dan overflow tidak otomatis aktif. Tabel ringkas harus disilangperiksa dengan uraian utama dan distribusi sebenarnya.
- **Turbo Vision [S4]:** kompatibilitas pustaka pengguna mencakup objek, event, view, dialog, validasi, collections, streams, dan resources. Membuat UI OTP terlihat serupa belum memenuhi kompatibilitas program Pascal yang memakai Turbo Vision.
- **UTILS.DOC [S5]:** petunjuk utilitas mencakup TOUCH, GREP, BINOBJ, dan TPUMOVER. Salinan ini masih perlu dicocokkan dengan distribusi acuan yang dipilih.

Temuan mutu sumber: sebuah daftar daring berlabel Turbo Pascal 7 ternyata berisi komponen Turbo Pascal for Windows. Sumber itu dikeluarkan dari spesifikasi DOS. Hasil OCR juga bisa merusak simbol seperti `$`, angka, dan tombol panah. tabel yang menjadi kontrak harus diperiksa terhadap halaman scan dan perilaku program.

Penelusuran lanjutan menemukan kandidat distribusi TP 7.0 English 1992 serta salinan FILELIST.DOC yang memuat TURBO/TPX/TPC, help TPH, TEMC, dan MANUAL.TV [S12–S13]. TEMC dan koreksi manual ditambahkan ke audit. README dari mirror yang sama menyebut Turbo Pascal 7.0. TEMC.DOC dan MANUAL.TV juga berhasil diambil dan bagian pembukanya diperiksa [S18]. Identitas release pada dokumen belum membuktikan bahwa seluruh binary mirror merupakan build asli yang tidak dipatch.

Belum terverifikasi: hash dan keaslian build distribusi, seluruh isi help/release notes, inventaris setiap simbol, perilaku error ambigu, serta cacat terkait kecepatan/perangkat. Materi tersebut dicari dan diverifikasi pengembang selama G0/G1. Tidak dianggap otomatis kompatibel.

## 3. Kontrak kompatibilitas native

| Lapisan | Bukti kelulusan |
|---|---|
| Bahasa | Program TP 7.0 sah diterima dan hasilnya cocok. Tipe, ukuran data, urutan evaluasi, overflow, parameter, unit, dan objek diuji. |
| Library | Seluruh simbol dalam profil TP 7.0 memiliki kontrak parameter, hasil, efek samping, kondisi batas dan kegagalan. |
| UI | Urutan input menghasilkan state dokumen, fokus, menu, dialog dan tampilan area klien yang setara. Nama dan logo diganti ODIN Turbo Pascal. |
| Debugger | Step, breakpoint, watch, evaluate/modify dan tampilan output diuji pada executable native. |
| Program hasil compiler | Header PE32+/ELF64/Mach-O dan instruksi sesuai target. Program dijalankan oleh loader OS tanpa launcher DOS atau kebutuhan memasang OTP. Library sistem standar masih diperbolehkan. |
| Data lama | Representasi file data Pascal dijaga. Pointer dan layout data TP tidak otomatis mengikuti ukuran pointer host. |
| Fungsi terikat DOS | Semantik dipertahankan melalui layanan kompatibilitas yang dibangun ke runtime native, dengan profil perangkat yang jelas. |
| Distribusi OTP | Clean install, penggunaan offline, compile/run, update dan uninstall lulus pada empat target wajib. |

Ada perbedaan antara kompatibilitas sumber dan kompatibilitas binary unit. File TPU/TPL/OBJ 16-bit Borland tidak bisa langsung disamakan dengan object native. Importer/bridge diteliti tersendiri. Jika importer belum tersedia, OTP harus memberi diagnostik yang jelas dan meminta sumber unit untuk dikompilasi ulang. Tidak menyebut format unit OTP sendiri sebagai TPU Borland. Impor binary DOS bukan dasar klaim kesamaan bahasa/library/UI yang dipilih pengguna.

Profil TP untuk uji membekukan bahasa antarmuka, compiler/IDE, switch awal, DOS, CPU/FPU, memori, adapter video, code page, input, clock dan filesystem. Compiler Borland menjadi pembanding hasil semantik. Executable OTP diuji pada setiap OS native.

Untuk kode `asm`, `inline`, `Mem`, `Port`, dan interrupt lama, bentuk sumber tetap menjadi pekerjaan kompatibilitas. Instruksi 16-bit tidak dieksekusi langsung sebagai instruksi ARM64 atau x86_64. Translasi atau bridge mesin terisolasi dipakai untuk bagian tersebut. Jika belum didukung, statusnya gagal/terblokir, bukan hasil sukses semu.

Perbandingan visual dilakukan pada framebuffer klien sebelum pembesaran DPI/Retina. Dekorasi window OS berada di luar area klien. Layar penuh disediakan untuk tampilan klasik.

Pernyataan rilis menggunakan profil, corpus dan hasil uji tertentu. Tidak mengubah status belum diuji menjadi lulus. Perbedaan yang disepakati: merek, logo, format executable native, ABI host internal, dan instalasi modern.

## 4. Arsitektur Rust untuk executable native

Aliran: sumber Pascal → lexer/preprocessor → parser → analisis tipe/scope → IR bersemantik TP → generator kode x86_64 atau ARM64 → object/linker target → executable native beserta runtime OTP.

| Target | Format program | ABI host yang dijembatani runtime |
|---|---|---|
| Windows x86_64 | PE32+, ekstensi .exe | Microsoft x64 |
| Linux x86_64 | ELF64 | System V AMD64 |
| macOS Intel | Mach-O x86_64 | Darwin x86_64 |
| macOS Apple Silicon | Mach-O arm64 | Darwin ARM64 |

Rujukan format memakai spesifikasi Microsoft, Linux Foundation, dan header resmi Apple [S14–S15, S17]. ABI TP di tingkat sumber dipisahkan dari ABI fungsi OS. Format .app adalah bundle untuk GUI macOS, sedangkan executable di dalamnya adalah Mach-O.

| Komponen Rust | Tanggung jawab |
|---|---|
| `otp-profile`, `otp-target` | Semantik TP, opsi, arsitektur, format, ABI, deployment minimum. |
| `otp-source`, `otp-frontend` | Byte sumber, include, token, parser, scope, tipe, konstanta, unit dan diagnostik. |
| `otp-ir`, `otp-codegen-x64`, `otp-codegen-arm64` | IR, urutan efek samping dan pembentukan kode mesin native. |
| `otp-object`, `otp-link` | Object dan executable PE/ELF/Mach-O, simbol, relokasi, import sistem dan informasi debug. |
| `otp-numeric` | Integer TP, Real48, Extended dan operasi numerik deterministik lintas host. |
| `otp-rtl` | Runtime Rust native, API System/Strings, heap, startup, I/O dan terminasi. |
| `otp-compat-memory`, `otp-dos`, `otp-machine16` | Model alamat TP, layanan DOS virtual, perangkat, bridge assembler lama yang diperlukan. |
| `otp-crt`, `otp-graphics`, `otp-tv` | CRT, Graph/BGI, Turbo Vision dan perangkat tampilan runtime native. |
| `otp-debug`, `otp-debug-agent` | Source map, kontrol proses, breakpoint, step dan inspeksi state. |
| `otp-ui`, `otp-editor` | Grid karakter, widget klasik, menu/dialog dan dokumen bersama. |
| `otp-platform` | Window, keyboard/mouse, clipboard, audio, printer, filesystem dan lifecycle OS. |
| `otp-cli`, `otp-ide`, `otp-tools` | Compiler CLI, IDE, dan utilitas. |
| `otp-conformance`, `xtask` | Corpus, differential tests, replay, packaging dan laporan. |

Native code generator dan linker sendiri merupakan rancangan implementasi yang harus dibuktikan pada G1. Tidak mengasumsikan seluruh backend native tersedia setelah frontend dibuat. Crate infrastruktur diperbolehkan sesuai keputusan, tetapi pemakaian compiler Pascal eksternal bukan implementasi compiler OTP. Jika komponen backend besar dari pihak ketiga diperlukan, catat perubahan arsitektur secara terbuka.

G1 boleh memakai linker OS sebagai alat pengembang untuk membuktikan object/ABI. Hasil percobaan tersebut tidak memenuhi syarat produk siap pasang. Sebelum rilis, pemasangan OTP harus menyertakan seluruh komponen kompilasi yang diperlukan tanpa meminta pengguna akhir memasang Xcode, Visual Studio, GCC, Rust, FPC atau Python. Linker, runtime, import library dan dependensi target perlu masuk manifest paket serta diuji offline.

Library Rust dibangun per target pada proses rilis OTP dan ditautkan ke program pengguna saat kompilasi. ABI antara kode Pascal dan library menggunakan konvensi yang terdokumentasi. Runtime yang ditautkan statis tidak berarti semua library OS ikut dibundel. Debug agent berkomunikasi dengan IDE melalui kanal lokal terstruktur. Program macet harus bisa dihentikan tanpa menghilangkan dokumen.

Semantik TP dipertahankan sebelum lowering native. Integer tetap 16-bit dalam profil TP, bukan ukuran word host. Real tetap 48-bit. Pointer TP dimodelkan sebagai alamat virtual kompatibel, tidak dijadikan pointer host mentah. Operasi library, indexing, pointer arithmetic dan konversi diuji bersama. `SizeOf` dan layout file tidak boleh berubah hanya karena host 64-bit.

Kode Pascal biasa menghasilkan instruksi native. Hanya operasi perangkat/assembler lama yang memerlukan bridge khusus. Menyimpan seluruh bytecode atau EXE DOS dalam launcher tidak memenuhi kontrak native ini.

Runtime CRT/Graph/Turbo Vision memetakan input dan tampilan ke jendela host sambil menjaga perilaku API yang terukur. Turbo Vision pengguna harus mendukung turunan objek, method override, callback, collections dan stream/resource. UI IDE sendiri memiliki pengujian terpisah.

Renderer menggambar sel/bitmap sendiri. Kandidat infrastruktur tetap `winit` dan `softbuffer` [S7–S8]. Palet dan ukuran sel mengikuti profil TP. Logo baru dipakai di splash/About dan identitas produk, tidak mengubah seluruh editor menjadi tema logo.

Layanan DOS memakai pemetaan drive/perangkat yang terdokumentasi. Program native mendapat kemampuan filesystem yang sesuai dengan operasi pengguna. `Exec` membedakan executable native dan DOS. Kode DOS tidak dijalankan sebagai proses native tanpa bridge. Tidak membuat stub yang selalu mengembalikan sukses untuk API yang belum diimplementasikan.

## 5. Daftar kerja kompatibilitas awal

Tabel ini adalah rancangan kelompok pekerjaan, bukan inventaris final setiap API/menu/instruksi. Setiap kelompok akan dipecah menjadi persyaratan atomik setelah distribusi dipilih. Seluruh kelompok berstatus belum diimplementasikan dan belum lulus.

| ID | Kelompok yang harus dicatat | Uji penerimaan yang direncanakan |
|---|---|---|
| C01 | Sumber byte, baris, komentar, identifier, literal | Corpus positif/negatif. posisi error. round-trip byte. |
| C02 | Include dan kompilasi kondisional | Include bertingkat, file hilang, cabang diabaikan, perubahan switch. |
| C03 | Seluruh direktif dan default | Matriks efek opsi lokal/global, CLI, konfigurasi, dan sumber. |
| C04 | Scope, resolusi nama, deklarasi | Benturan nama, nested scope, urutan deklarasi, symbol lookup. |
| C05 | Integer, ordinal, enum, subrange | Nilai batas, operasi, promosi, ukuran, dan error. |
| C06 | Real dan model FPU | Pola bit, pembulatan, konversi, format output, kondisi luar rentang. |
| C07 | String, karakter, PChar | Panjang batas, assignment, parameter, encoding, terminator. |
| C08 | Array, record, set, file, pointer | Layout byte, alignment, indexing, alias, operasi himpunan. |
| C09 | Ekspresi dan alur kontrol | Urutan evaluasi, short circuit, case, loop, with, goto. |
| C10 | Prosedur/fungsi dan parameter | Rekursi, nested calls, hidden parameters, aliasing dan konvensi ABI. |
| C11 | Object Pascal versi acuan | Inheritance, dispatch, constructor/destructor, kegagalan konstruksi. |
| C12 | Unit dan dependency | Interface/implementation, inisialisasi, rebuild, dependency berubah. |
| C13 | Assembler, inline, external | Instruksi, register/flags, simbol, stack, relokasi, kode campuran. |
| C14 | Layout dan batas memori | Segment crossing, heap/stack exhaustion, alamat/offset, absolute. |
| C15 | Diagnostik compiler/runtime | Kode, teks, lokasi, urutan error, dan status keluar. |
| C16 | Compiler CLI, make/build | Parsing opsi, precedence, output, status, perubahan berkas. |
| L01 | Semua simbol setiap unit RTL | Kontrak ABI dan efek samping. satu rekaman untuk setiap simbol. |
| L02 | Startup, heap, terminasi | Inisialisasi, cleanup, error runtime, handler, exit status. |
| L03 | I/O teks dan biner | EOF, short read/write, buffer, typed/untyped file, perangkat gagal. |
| L04 | CRT dan input | Kursor, atribut, scroll, tombol extended, sound, waktu, layar. |
| L05 | Layanan DOS dan printer | File attributes, pencarian, tanggal, environment, proses, output spool. |
| L06 | Overlay | Pemanggilan lintas overlay, buffer, kegagalan loading, disk/EMS bila acuan memerlukannya. |
| L07 | Graph/BGI/CHR | Driver/mode/font acuan, operasi raster, clipping, palette, error. |
| L08 | Unit kompatibilitas lama | Corpus khusus setiap unit yang benar-benar ada dalam distribusi. |
| L09 | Turbo Vision publik | Seluruh objek, field, konstanta, event, method, collections, stream/resource. |
| L10 | Ekstensi/pustaka contoh yang disertakan | Manifest membedakan unit standar, contoh, tool tambahan, dan paket terpisah. |
| F01 | Executable native | Header/ISA/ABI PE32+, ELF64, Mach-O, import, relokasi, startup, exit dan runtime lulus pada OS target. |
| F02 | TPU/TPL dan unit native | Kompilasi ulang unit dari sumber wajib. Import binary Borland adalah riset interoperabilitas terpisah, dengan status dukungan eksplisit. |
| F03 | Object native dan OMF lama | Linking native diuji per ABI. External object 16-bit memerlukan translator/bridge dan tidak dianggap langsung kompatibel. |
| F04 | BGI dan CHR | Baca aset acuan yang tersedia. hasil grafik dan font dibandingkan. |
| F05 | Overlay dan data program | API/perilaku overlay dipetakan ke runtime native. Data Pascal dibandingkan bytewise. Pertukaran OVR binary lama dilaporkan tersendiri. |
| F06 | TP/CFG, desktop, TPH/help, debug/map | Inventaris format per versi. import/export diuji menurut kontraknya. |
| M01 | CPU dan instruksi yang relevan | Register, flag, segment wrap, interrupt, illegal instruction. |
| M02 | FPU | Instruksi, state/status, presisi, pembulatan, kondisi error. |
| M03 | DOS/BIOS dan loader | Trace layanan, environment/proses, vector interrupt, memory allocation. |
| M04 | Video, keyboard, mouse | Replay input dan checksum framebuffer/state pada profil perangkat. |
| M05 | Timer, speaker, clock | Clock virtual deterministik serta mode realtime. perilaku kode timing. |
| M06 | File dan perangkat guest | Semantik drive, nama pendek, case, atribut, error, batas path. |
| M07 | Shell dan program eksternal | Parameter, environment, hasil keluar, kembali ke IDE, output tertangkap. |
| U01 | Seluruh menu/submenu/popup | Label, mnemonic, posisi, state enabled/disabled, urutan fokus. |
| U02 | Dialog dan widget | Tab order, default/cancel, history, validasi, mouse/keyboard. |
| U03 | Desktop dan jendela | Move, resize, zoom, tile, cascade, close, focus dan restore. |
| U04 | Dokumen dan editor | Beberapa view berbagi dokumen. state kursor, block dan undo diuji. |
| U05 | Keybinding dan TEMC | Replay shortcut/chord, konfigurasi/makro editor, dan benturan dengan shortcut OS. |
| U06 | Search/replace | Pilihan pencarian, ekspresi reguler acuan, batas dan pembatalan. |
| U07 | Save/load/print | Encoding, line endings, backup, gagal simpan, format cetak. |
| U08 | Compile/run/build dalam IDE | State dokumen, destination, argument, error jump, output, kembali edit. |
| U09 | ObjectBrowser dan simbol | Navigasi, referensi, unit/objek, persistensi dan kompilasi gagal. |
| U10 | Tools dan Messages | Konfigurasi, parameter macro, pemetaan error ke sumber, exit status. |
| U11 | Bantuan | Context ID, navigasi, history/index, contoh. format dan konten dipisahkan. |
| U12 | Konfigurasi/persistensi | Round-trip opsi, desktop, posisi, simbol. startup setelah file rusak. |
| D11 | Step/trace/run to cursor | Lokasi berhenti sumber, panggilan nested, kode eksternal. |
| D12 | Breakpoint | Kondisi, hit count, enable/disable, lokasi setelah rebuild. |
| D13 | Watch/evaluate/modify | Tipe dan format nilai, pointer/object, efek modifikasi terhadap eksekusi. |
| D14 | Output dan state debug | Pertukaran layar, reset, terminate, stack dan error guest. |
| T01 | Utilitas dalam manifest acuan | Perbandingan argumen, transformasi file, output/error untuk tiap tool. |
| T02 | Tools produk lain | Catat ketergantungan dan keputusan scope. keberadaan menu tidak berarti produknya sudah ditulis ulang. |
| P01 | Windows | Clean install/offline, IDE/CLI, file association opsional, update/uninstall. |
| P02 | Linux | X11/Wayland, filesystem case, desktop integration, paket dan dependensi sistem. |
| P03 | macOS Intel | Native x86_64, keyboard/Fn, Retina, lifecycle, signing dan packaging. |
| P04 | macOS Apple Silicon | Native arm64, kesamaan hasil dengan Intel, tanpa mengandalkan Rosetta. |
| P05 | Ketahanan data | Crash recovery, atomic save, cancel, disk penuh, program loop tak berujung. |
| P06 | Rilis dan reproducibility | Build terkunci, manifest/hash, lisensi, smoke test artefak yang benar-benar dikirim. |

Backend/linker native, model memori TP, M02, C13 dan L09 merupakan risiko besar. F02/F03 membedakan kompilasi sumber yang wajib dari impor binary DOS yang memerlukan bridge. Perbedaan ini mengikuti keputusan keluaran native dan kontrak kesamaan bahasa/library/UI, bukan bukti bahwa importer sudah dibuat. Semua gap tetap dicatat.

## 6. Pengendalian agar kebutuhan tidak terlewat

Setiap kebutuhan atomik diberi rekaman: `id`, versi/profil, sumber dan halaman/topik, stimulus, state awal, hasil yang diharapkan, implementasi, test ID, hasil aktual, status, dan keputusan perbedaan. Daftar simbol, menu, tombol, format, instruksi dan berkas menjadi indeks terpisah, bukan paragraf yang sulit dihitung.

Tahapan audit kelengkapan:

1. Hash seluruh berkas distribusi acuan dan tentukan manifest mana yang berlaku.
2. Cocokkan setiap berkas terhadap fungsi: executable, unit, aset, konfigurasi, help, contoh, atau dokumen.
3. Silangkan daftar isi manual, indeks API, help, README, FILELIST, dan perilaku program.
4. Pecah setiap entri publik menjadi kontrak dan test. Duplikasi dan konflik sumber diselesaikan, bukan dibuang.
5. Jalankan percobaan kecil pada implementasi Borland untuk perilaku yang tidak jelas. simpan input, environment, output, dan hash hasil.
6. Review daftar entri tanpa test, format tanpa fixture, menu tanpa replay, serta berkas tanpa padanan.
7. Bekukan denominator persyaratan sebelum melaporkan tingkat kelulusan. Kebutuhan baru menambah denominator. tidak mengubah hasil gagal menjadi “tidak relevan” tanpa keputusan pemilik.

Status yang diperbolehkan: ditemukan, terspesifikasi, terimplementasi, teruji, lulus, gagal, terblokir, atau di luar scope dengan persetujuan eksplisit. Saat ini baru riset dan rancangan kelompok. belum ada persentase kompatibilitas yang sah.

## 7. Tahap pembangunan dan gerbang penerimaan

| Tahap | Hasil | Gerbang |
|---|---|---|
| G0 | Keputusan produk, profil, inventaris simbol/menu/file, corpus dan brand | Keputusan D01–D07 tercatat. Bahan acuan dipilih dan status verifikasinya jelas. |
| G1 | Compiler minimal Rust, object/linker/RTL native, jendela host, probe legacy | Hello/input/aritmetika/file menjadi native pada empat target. Tidak lulus hanya karena binary punya ekstensi yang benar. |
| G2 | Bahasa, unit, object Pascal, numerik, diagnostic, assembler bridge | Corpus sumber positif/negatif lulus sesuai cakupan tahap. |
| G3 | Library dan compatibility runtime | Semua simbol RTL mempunyai test. Hasil dibandingkan dengan Borland DOS dan konsisten lintas host. |
| G4 | Graph/BGI, Turbo Vision, overlay, data | Program grafis/UI pengguna serta data lama lulus. |
| G5 | IDE, debugger native, ObjectBrowser, Tools, help dan TEMC | Replay UI serta sesi debug lulus di seluruh host. |
| G6 | Audit kompatibilitas dan hardening | Tidak ada persyaratan wajib yang gagal/tidak diuji. Importer lama dilaporkan berdasarkan dukungan nyata. |
| G7 | Installer, signing, dokumentasi dan corresponding source GPLv3 | Clean install dan compile/run offline lulus tanpa toolchain eksternal. Program hasil kompilasi lulus pada mesin tanpa OTP. |

Gerbang G1 tidak dinyatakan selesai sebelum empat target benar-benar diuji. Jika lingkungan kerja hanya Linux, laporkan hasil Linux saja dan tandai Windows/macOS belum diverifikasi. Perkiraan jadwal setelah pengukuran G1, bukan janji kompatibilitas berdasarkan jumlah baris kode.

## 8. Strategi verifikasi

Pembanding dipisahkan: program yang dikompilasi Borland dan dijalankan pada DOS acuan, serta program sumber yang sama dikompilasi OTP dan dijalankan native pada setiap OS. Hasil dibandingkan pada semantik, file data, error dan tampilan. Header, disassembly dan dependensi memverifikasi bahwa hasil OTP benar-benar native. FPC/Delphi hanya pembanding tambahan, bukan penentu semantik TP 7.0.

Jenis bukti: keluaran stdout/stderr, exit code, file biner, trace layanan, snapshot memori/register, pola bit numerik, state debugger, replay input, dan framebuffer. Tanggal/waktu/random hanya dinormalisasi bila tidak sedang diuji. jangan menghapus perbedaan yang merupakan perilaku nyata.

Skenario wajib meliputi program konsol, file data, aplikasi CRT, multi-unit, objek/polimorfisme, Turbo Vision, BGI, assembler/OBJ, overlay, overflow/range/I/O error, memory exhaustion, salah sintaks, berkas rusak, dan penghentian program macet. Corpus pengguna ditambahkan dengan izin akses yang sesuai.

Pengujian lintas OS mencakup jalur ber-spasi/non-ASCII, CRLF/LF, keyboard internasional, F1–F12/Fn/Alt/Option, mouse, DPI/Retina, layar ganda, minimize/resume, clipboard, filesystem case, audio/printer, serta instalasi tanpa internet. Perilaku legacy yang bertentangan dengan kenyamanan host dicatat sebagai pilihan profil.

Fuzzing parser/loader penting karena kode dan berkas guest bisa rusak. Miri/sanitizer dapat dipakai pada komponen yang sesuai. Uji performa menilai respons editor dan batas penghentian guest. angka target baru dibakukan setelah hardware uji ditentukan.

## 9. Platform dan paket rilis

Berikut usulan batas aplikasi, bukan jaminan yang sudah diuji. Dukungan compiler Rust terhadap target tidak otomatis membuktikan dukungan seluruh dependensi atau aplikasi [S6].

| Host wajib | Arsitektur | Usulan minimum awal | Paket |
|---|---|---|---|
| Windows | x86_64 | Windows 10/11. versi uji spesifik dibekukan | Installer MSI atau EXE, ditambah ZIP portable |
| Linux | x86_64 | Baseline Ubuntu 22.04 atau yang disepakati | DEB, AppImage, TAR. RPM bila diminta |
| macOS Intel | x86_64 | macOS 12 atau yang disepakati | APP dalam DMG |
| macOS Apple Silicon | arm64 | macOS 12 atau yang disepakati | APP dalam DMG. Universal 2 jika seluruh komponen teruji |

Linux ARM64, Windows ARM64, dan Windows 32-bit belum termasuk usulan wajib. Tambahkan jika “semua Windows/Linux” dimaksudkan juga mencakup arsitektur tersebut. macOS Intel dan Apple Silicon tetap wajib sesuai permintaan awal.

Paket memuat compiler native, linker, IDE, library native per target, compatibility runtime, aset, help offline, contoh, metadata versi, panduan dan lisensi. Program hasil kompilasi membawa runtime yang diperlukan atau paket pendamping yang eksplisit. Penerima program tidak harus memasang OTP. Pengguna OTP tidak harus memasang toolchain pengembang.

Rilis macOS untuk distribusi normal memerlukan alur Developer ID/notarization yang sesuai [S9]. CI macOS dan kredensial signing menjadi prasyarat rilis, bukan bukti yang sudah tersedia saat ini. Windows memerlukan keputusan sertifikat penandatanganan. Semua rahasia signing berada di penyimpanan rahasia build, bukan source tree. Paket Linux tetap menyatakan kebutuhan library sistem aktual. “AppImage” tidak otomatis berarti setiap distro didukung.

## 10. Inventaris source dan berkas yang akan diserahkan

Ini manifest rencana. Berkas berikut belum dibuat sebagai implementasi Rust.

| Path/kelompok | Isi yang wajib tersedia |
|---|---|
| `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` | Workspace, dependensi terkunci, versi toolchain yang diuji. |
| `crates/<komponen>/Cargo.toml`, `src/*.rs` | Modul pada tabel arsitektur, tanpa logika compiler/library tersembunyi di layanan eksternal. |
| `profiles/*.toml` | Profil compiler, mesin, UI, batas, dan target. |
| `spec/requirements.json`, `spec/symbols.json`, `spec/formats/` | Kontrak atomik, ABI, layout, format, dan sumber bukti. |
| `spec/decisions/` | Keputusan arsitektur dan semua pengecualian yang disetujui. |
| `library/spec/`, `library/native/<target>/` | Definisi API, deklarasi Pascal, unit/runtime Rust yang dibangun untuk tiap ABI native. |
| `assets/fonts/`, `assets/icons/`, `assets/palettes/` | Aset milik OTP atau yang berizin, berikut provenance. |
| `help/`, `docs/`, `examples/*.pas` | Bantuan offline, manual Indonesia, referensi API, contoh milik OTP. |
| `tests/corpus/`, `tests/fixtures/`, `tests/replay/` | Kasus compiler/runtime/format/UI, hasil yang diharapkan, profil dan hash. |
| `tests/oracle/` | Harness untuk pembanding berlisensi yang dipasang terpisah. tanpa membundel compiler asli. |
| `packaging/windows/`, `packaging/linux/`, `packaging/macos/` | Definisi installer, desktop integration, ikon, manifest dan entitlement seperlunya. |
| `.github/workflows/` atau padanan CI | Build/test per OS dan arsitektur, packaging, signing, artefak dan laporan. |
| `README.md`, `BUILD.md`, `INSTALL.md`, `COMPATIBILITY.md`, `CHANGELOG.md` | Cara membangun, memasang, memakai, batas dan riwayat rilis. |
| `LICENSE`, `THIRD_PARTY_NOTICES`, `SBOM`, `SHA256SUMS` | GPL-3.0-only untuk source OTP, pemberitahuan dependensi, inventaris komponen dan integritas. |
| `dist/` | Installer yang benar-benar dibangun dan diuji. tidak diisi placeholder lalu disebut siap pasang. |

Kode produk ditulis Rust. Berkas konfigurasi, metadata installer, dokumentasi, fixture, deklarasi dan contoh Pascal tetap diperlukan. “seluruhnya Rust” tidak berarti setiap berkas harus berekstensi `.rs`. Tidak mengandalkan port langsung unit Borland sebagai implementasi baru.

## 11. Risiko dan batas rilis

| Risiko | Penanganan |
|---|---|
| Hasil berupa launcher interpreter, bukan native | Periksa disassembly fungsi Pascal, header, symbol map dan dependency. Tidak mengesahkan wrapper EXE DOS sebagai compiler native. |
| Integer/pointer/layout berubah mengikuti host | Model tipe dan memori TP eksplisit serta test ABI/data. |
| Native linker mengandalkan alat yang tidak ada di mesin pengguna | Bundle komponen kompilasi dalam installer dan uji compile offline pada mesin bersih. |
| 16-bit assembler di ARM64 | Translator/bridge dengan uji register, flag, memory dan callback. Dukungan parsial diberi diagnostik. |
| API DOS/BGI hanya berupa stub | Test efek nyata, hasil/error, framebuffer dan file. Semua API wajib mempunyai kontrak. |
| Presisi numerik/timing berbeda | Model numerik/clock dan differential test. |
| Format TPU/TPL/help tidak terdokumentasi | Bedakan kompatibilitas sumber dari impor binary. Tulis dukungan per versi dan fixture. |
| Turbo Vision tampak mirip tetapi ABI salah | Test subclass Pascal, callback, collections, stream/resource pengguna. |
| Logo mengubah tampilan editor | Batasi brand pada splash/About/ikon. Grid dan palet editor tetap acuan DOS. |
| Mirror TP 7.0 ternyata patched/7.01 | Validasi versi, checksum, README dan provenance sebelum memakai binary sebagai oracle. |
| GPL compiler disamakan dengan lisensi setiap hasil | Catat lisensi runtime yang ditautkan dan distribusi source. Jangan membuat pengecualian lisensi tanpa keputusan. |
| Build sukses tetapi paket tidak berjalan | Uji installer dan program output pada masing-masing OS/arsitektur secara native. |

Materi Borland yang ditemukan dipakai sebagai acuan terpisah. Kode, logo, bantuan, dan aset produk OTP dibuat sendiri. Bahan yang bisa diunduh dari arsip tidak otomatis ditambahkan ke distribusi GPL OTP.

## 12. Sumber dan tingkat peninjauan

Nomor halaman manual, bila digunakan pada tahap berikutnya, harus membedakan halaman cetak dan indeks halaman PDF. Tautan merupakan lokasi salinan. pengarang dokumen asli tetap Borland.

| ID | Sumber | Pemakaian/tingkat bukti |
|---|---|---|
| S1 | [Turbo Pascal 7.0 User’s Guide, Borland 1992](https://turbopascal.nl/docs/Turbo_Pascal_Version_7.0_Users_Guide_1992.pdf) | 188 halaman PDF. Bagian IDE, compiler workflow, debugging, konfigurasi, dan daftar berkas rujukan ditinjau. |
| S2 | [Turbo Pascal 7.0 Language Guide, Borland 1992](https://turbopascal.nl/docs/Turbo_Pascal_Version_7.0_Language_Guide_1992.pdf) | 319 halaman PDF. Tipe/numerik, unit, runtime, layout, dan assembler ditinjau pada bagian terkait. |
| S3 | [Turbo Pascal 7.0 Programmer’s Reference, Borland 1992](https://turbopascal.nl/docs/Turbo_Pascal_Version_7.0_Programmers_Reference_1992.pdf) | 310 halaman PDF. Struktur referensi API, opsi, error, editor, dan tabel direktif ditinjau. |
| S4 | [Turbo Vision 2.0 Programming Guide, Borland 1992](https://archive.org/details/bitsavers_borlandTurrogrammingGuide1992_25707423) | 640 halaman PDF. Daftar isi, hierarki/event, collections, streams/resources, validasi, dan struktur referensi ditinjau. |
| S5 | [Salinan UTILS.DOC Borland](https://github.com/keskival/turbo-pascal-experiments/blob/master/paketti/DOC/UTILS.DOC) | Petunjuk utilitas. kecocokan build/distribusi belum dipastikan. |
| S6 | [Rust platform support](https://doc.rust-lang.org/rustc/platform-support.html) dan [Apple Darwin targets](https://doc.rust-lang.org/rustc/platform-support/apple-darwin.html) | Dokumentasi resmi target host. bukan bukti bahwa OTP sudah mendukungnya. |
| S7 | [winit, repositori pengembang](https://github.com/rust-windowing/winit) | Kandidat window/input. bukan dependensi yang sudah dipatok. |
| S8 | [softbuffer, repositori pengembang](https://github.com/rust-windowing/softbuffer) | Kandidat presentasi framebuffer CPU lintas OS. |
| S9 | [Apple Developer ID](https://developer.apple.com/developer-id/) | Rujukan distribusi macOS/signing/notarization. |
| S10 | [Intel 8086 Family User’s Manual, 1979](https://bitsavers.org/components/intel/8086/9800722-03_The_8086_Family_Users_Manual_Oct79.pdf) | Rujukan lanjutan untuk model mesin. belum diaudit pada tahap ini. Dokumen 286/FPU dan BIOS/perangkat harus ditambahkan sesuai profil. |
| S11 | [Microsoft MS-DOS source archive](https://github.com/microsoft/MS-DOS) | Bahan lanjutan untuk layanan DOS. bukan pengganti otomatis versi DOS pembanding dan belum diaudit di tingkat implementasi. |
| S12 | [FILELIST.DOC, mirror dokumen distribusi](https://github.com/fxia22/ASM_xf/blob/master/TP/FILELIST.DOC) | Manifest kandidat DOS. TEMC, help TPH dan MANUAL.TV ditemukan. Belum dipastikan build persisnya. |
| S13 | [Katalog arsip Turbo Pascal 7.x](https://winworldpc.com/product/turbo-pascal/7x) | Menunjuk paket 7.0 English 1992 dan membedakannya dari 7.01. Katalog arsip, bukan otoritas semantik bahasa. Binary belum dieksekusi/diuji pada tahap ini. |
| S14 | [Microsoft PE/COFF](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format) | Spesifikasi format keluaran Windows. |
| S15 | [System V ELF ABI](https://refspecs.linuxfoundation.org/elf/gabi4+/contents.html) | Format ELF. ABI prosesor perlu dikunci dalam implementasi. |
| S16 | [GPL-3.0-only, SPDX](https://spdx.org/licenses/GPL-3.0-only.html) dan [teks GNU GPLv3](https://www.gnu.org/licenses/gpl-3.0.txt) | Identifier, teks GPL versi 3 dan aturan output pada bagian 2. |
| S17 | [Apple Mach-O loader header](https://github.com/apple-oss-distributions/xnu/blob/main/EXTERNAL_HEADERS/mach-o/loader.h) | Rujukan format Mach-O resmi. |
| S18 | [README](https://raw.githubusercontent.com/fxia22/ASM_xf/master/TP/README), [TEMC.DOC](https://raw.githubusercontent.com/fxia22/ASM_xf/master/TP/DOC/TEMC.DOC), [MANUAL.TV](https://raw.githubusercontent.com/fxia22/ASM_xf/master/TP/DOC/MANUAL.TV) | Dokumen Borland pada mirror yang sama. Judul README TP 7.0, macro/keybinding editor, tambahan dan koreksi Turbo Vision. Bagian pembuka diperiksa. |

## 13. Identitas visual dan status pekerjaan

Nama: ODIN Turbo Pascal. Singkatan: OTP.

Logo dibuat dari awal. Potret Odin berjanggut dan bermata tertutup sebelah menjadi tokoh mentor. Petir menyatu dengan siluet. Buku terbuka memuat token `begin` dan `end.` sebagai penanda belajar Pascal. Petir dipakai sebagai simbol kreatif merek, bukan pernyataan sejarah mitologi.

Gaya mengikuti lampiran: bitmap/dithering era 1990-an, hitam, kuning terang, putih dan magenta terbatas. Tulisan brand berdiri sendiri dan terbaca. Wajah Pascal, tulisan Borland, watermark serta copyright pada gambar acuan tidak disalin.

Keluaran logo saat ini: satu master raster persegi dengan wordmark dan latar hitam. Path proyek: `assets/branding/odin-turbo-pascal-master-v1.png`. Ini cocok untuk identitas/splash. Varian ikon kecil, ICO, ICNS dan ukuran aplikasi akan diturunkan pada tahap packaging, lalu diuji keterbacaannya. Belum disebut aset vektor.

Cara pembuatan: imagegen bawaan, dengan lampiran sebagai referensi gaya. Ringkasan prompt: original Odin mentor portrait, eyepatch, lightning silhouette, open Pascal learning book with begin/end., ODIN Turbo Pascal wordmark, black/electric-yellow pixel dithering, white pixel typography, sparse magenta accents, no copied Borland portrait/logo/watermark. Prompt produksi: “Create one original ODIN Turbo Pascal logo. Use the supplied screenshot only as a style reference for black/electric-yellow high-contrast early-1990s bitmap art. Draw a new wise Odin mentor with an eyepatch, geometric beard, lightning silhouette and an open learning book showing begin and end. Use white pixel typography for ODIN and Turbo Pascal, sparse magenta coding accents, solid black background and safe margins. Keep hard pixel edges and dithering. Do not copy the original portrait, Borland logo, watermark or copyright. No gradients, 3D, mockup or extra slogans.”

Keputusan 1–5 pengguna sudah diterapkan. Tidak ada pertanyaan keputusan yang sama diajukan kembali. Minimum OS mengikuti tabel usulan sampai hasil uji dependensi menentukan batas aktual. Lisensi kerja GPL-3.0-only.

Selesai pada revisi ini: penguncian spesifikasi, perubahan arsitektur dari keluaran DOS menjadi native, penambahan sumber pembanding, dan satu desain logo asli. Belum selesai: implementasi compiler Rust, library, IDE dan installer. Langkah implementasi terdekat adalah G0 lanjutan dan pembuktian G1 untuk empat target native.


## Lampiran. Identitas bahan yang sudah diambil

Hash di bawah mengidentifikasi berkas yang dipakai selama riset. Hash bukan bukti keaslian penerbit. Materi manual dan binary Borland tidak disertakan ke distribusi OTP.

| Berkas riset | SHA-256 |
|---|---|
| UG7.pdf | a9a0340414ee46133fcd2059c1e595049cbecffe68689f2c87e216da619b868f |
| LG7.pdf | 0c92e22de98f31377b6efaf89086bbe0c40feccaed652b83aa9e301cf680f608 |
| PR7.pdf | a5c716c18ef11d630997aa064da1ba03c8af3b4fc551ef9ad92665aa8d9e741d |
| TV2.pdf | 614a72564747b3f68eb78496249c5fa9c0d75858a851152b4e933b559f650650 |
| TP_FILELIST.DOC | 020170c842e30b28b67ccacd0b2ada56f3dda3fc77c51dd3a896c8d045e07f85 |
| TP_README | 17871f63be5eb952267f22a86086e4051c87aeb9d8273f3217bdbdbb0f1d1e56 |
| TP_TEMC.DOC | 162dfeb0e29a8f9d8943ba7238e6850efc2f22d32ba0eabcac33372e8a32c52a |
| TP_MANUAL.TV | 7251db9f7d4bc3dc2dd2ec776e632caeb5b8303687907169e020376d5b963de8 |
