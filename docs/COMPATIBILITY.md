# Matriks kompatibilitas v2.0, milestone dev.3

Acuan target: Borland Turbo Pascal 7.0 DOS English 1992. Status dokumen ini merujuk implementasi nyata saat ini. Dokumen RENCANA adalah target akhir, bukan daftar fitur yang sudah selesai. Belum ada sertifikasi atau persentase kesamaan 100%.

| Bagian | Tersedia pada milestone ini | Kesenjangan terhadap target |
|---|---|---|
| Lexical | Identifier case insensitive ASCII, angka desimal/hex, real eksponen, string kutip ganda Pascal, `#nn`, komentar `{}` dan `(* *)` | CP437 penuh, semua aturan lexical TP, recovery multi-error belum |
| Deklarasi | Program, const sederhana/ekspresi, type alias, var, procedure/function, parameter value/var, routine bersarang, rekursi | Typed constants, forward/external, procedural types, open array belum; beberapa aturan urutan deklarasi lebih longgar |
| Integer | Byte, ShortInt, Integer 16-bit, Word, LongInt 32-bit; wrap dan opsi pengecekan | Promosi signed/unsigned, literal batas, operasi shift dan semua kode error belum dipastikan identik TP |
| Real | Penyimpanan enam byte Real48; matematika dasar dan transcendental melalui Rust f64 | Intermediate f64 dibulatkan ke Real48; bukan aritmetika 8087/software TP yang telah dibuktikan bit-exact; Single/Double/Extended/Comp belum |
| Boolean / Char | Boolean, Char, Ord/Chr, UpCase ASCII, short-circuit Boolean | Semua promosi dan codepage TP belum; evaluasi lengkap `$B+` belum |
| String | Short string hingga 255 byte, kapasitas, concat, copy, pos, length, baca indeks termasuk length byte index 0 | Tulis `s[i]`, PChar/Strings unit, Str/Val/Delete/Insert belum; indeks selalu aman, bukan memori DOS mentah |
| Aggregate | Array bounds integer/char literal, multidimensi; record sederhana; salin value, parameter var | Batas ordinal bertipe enum/subrange, variant record, packed layout, WITH, identitas nominal tipe TP belum |
| Kontrol | Begin/end, if/else, while, repeat, for integer to/downto, case rentang, exit | FOR ordinal noninteger belum; pembatasan assignment control variable belum lengkap; GOTO/label belum; Break/Continue tersedia sebagai ekstensi |
| Sistem unit | `uses System, Crt, Dos, Strings` dikenali sebagai nama profil bawaan parsial | Belum ada compiler unit/interface/implementation, TPU/DCU atau namespace/export lengkap. Menulis `uses Dos` tidak berarti API Dos tersedia |
| System math | Abs, Sqr, Sqrt, Sin, Cos, ArcTan, Ln, Exp, Round, Trunc, Pi, Odd, Inc, Dec, Succ, Pred, Lo, Hi | Domain/type/overflow pada semua edge case belum TP-identik; Succ/Pred Boolean belum |
| Input/output | Write/WriteLn dengan width, real decimals; Read/ReadLn scalar; Assign, Reset, Rewrite, Close, Erase, Flush, Eof pada Text | Typed/untyped file, seek/block I/O, standard-input EOF belum; Eoln parsial; error I/O belum seluruhnya bernomor sama TP; newline mengikuti OS; input teks memakai UTF-8 host |
| Lainnya | ParamCount, ParamStr, Halt, Delay, Random, Randomize; IOResult bernilai 0 bila belum terjadi abort I/O | RNG bukan algoritma TP; `$I-`/IOResult recovery belum; input standar akhir-file menghasilkan error 106 |
| CRT | ClrScr, ClrEol, GotoXY, WhereX/Y, TextColor/Background melalui ANSI | ReadKey/KeyPressed, sound, windowing text, video memory, scrolling dan cursor/screen state lengkap belum. Output IDE tidak mengemulasi seluruh escape CRT |
| Direktif | `$R+/-`, `$Q+/-`, `$D+/-` sebelum token pertama, `{...}` atau `(*...*)` | Lokal/per-block, conditional compile, include, resource/link directives dan lainnya ditolak eksplisit |
| Pointer/memori | Tidak ada | Pointer, New/Dispose, GetMem/FreeMem, segment/offset, absolute, port/mem, interrupt, 16-bit memory model belum |
| Object Pascal | Tidak ada | Object, method, constructor/destructor, virtual dispatch, inheritance belum |
| Graph/BGI | Tidak ada | Driver, mode, palet, drawing, font CHR, input dan error BGI belum |
| Turbo Vision | Tidak ada | Objects, Views, Drivers, Dialogs, Menus, App, MsgBox, Validate, collections/streams dan lainnya belum |
| IDE | Jendela per dokumen, Tile/Cascade/Zoom, move/resize, scrollbar, daftar jendela, file browser dengan konfirmasi overwrite, open/save, undo/redo, selection, clipboard internal, find/replace, menu, compile/run, output/input, konfigurasi pemeriksaan sementara | Grid bukan 80×25; keybinding WordStar, TEMC, layout/event tepat, project/make, help TPH, konfigurasi persisten, OS clipboard belum |
| Debugger | Statement stepping, next, continue, breakpoint baris, snapshot nilai scalar, stop program | Watches ekspresi, evaluasi/ubah nilai, stack browser, kondisi breakpoint, register/assembly/DOS debugger belum |
| Native backend | Assembly x86_64 SysV/Win64/Darwin dan ARM64 Darwin; control flow dan procedure native | Keempat target (Linux x64, Windows x64, macOS x64, macOS ARM64) dilink dan dijalankan di CI native per host, native suite 48/48. Belum object writer/linker mandiri, optimisasi atau layout ABI TP |
| Paket | Source, vendored dependencies; Linux tar.gz, Windows ZIP, macOS app/DMG/tar.gz dengan installer per-user; paket npm `odin-turbo-pascal` + paket biner per platform, diuji install di CI | Belum MSI/EXE installer, AppImage/DEB/RPM, notarized/signature rilis, updater, universal macOS binary; paket npm belum diterbitkan ke registry |

## Perilaku internal yang perlu dipahami

1. Pascal diturunkan ke assembly berisi branch, call routine, dan panggilan helper Rust. Procedure Pascal memiliki fungsi native masing-masing. Nilai tersimpan dalam representasi bertipe terkelola pada runtime, bukan tata letak memori TP/DOS. Backend ini tidak mengeksekusi bytecode atau AST melalui interpreter.
2. Generator masih memakai helper runtime untuk operasi dan storage, sehingga kecepatan/ukuran kode belum menyerupai compiler Borland. Tujuan milestone pertama adalah executable yang bisa dijalankan dan dasar semantik yang dapat diuji.
3. Nilai lokal/global diinisialisasi nol/kosong untuk determinisme. Perilaku variabel TP yang belum diinisialisasi belum ditiru. Array/record dibandingkan secara struktural pada type checker.
4. Rentang indeks array/string selalu diperiksa untuk menjaga konsistensi runtime. `$R-` bukan izin untuk akses memory DOS yang tidak valid.
5. Error kompilasi dilaporkan bersama baris; kolom pada tahap semantic/codegen umumnya 1. Error string belum identik dengan daftar error Borland.
6. Direktif yang diterima sekarang bersifat seluruh program dan harus berada sebelum PROGRAM/token pertama. Direktif lokal ditolak, bukan diterapkan diam-diam secara global.
7. Routine builtin yang belum diimplementasikan ditolak sebagai unknown/unimplemented. Unit Graph dan Turbo Vision tidak diganti unit kosong.
8. Keyboard/clipboard/menu serta transcript UI masih merupakan implementasi pengembangan. Debugger CLI cocok untuk memeriksa state yang lebih panjang daripada panel IDE.

## Target berikutnya sesuai RENCANA

Urutan kerja: unit compiler dan namespace → model ordinal/typed constants/pointer serta layout → API System dan file I/O lengkap → object model/Objects → DOS/CRT compatibility layer → BGI → Turbo Vision → replay UI dan shortcut → differential corpus TP7 → linker mandiri, paket dan pengujian semua OS. Gerbang penerimaan di RENCANA tetap berlaku; hasil tes subset bukan pengganti gerbang tersebut.
