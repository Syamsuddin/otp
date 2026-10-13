{ SPDX-License-Identifier: GPL-3.0-only }
program Berkas;
var f: Text; baris: string;
begin
  Assign(f, 'contoh-otp.txt'); Rewrite(f);
  WriteLn(f, 'Belajar Pascal dengan ODIN');
  WriteLn(f, 'File teks lintas OS'); Close(f);
  Reset(f);
  while not Eof(f) do begin
    ReadLn(f, baris); WriteLn(baris);
  end;
  Close(f);
end.
