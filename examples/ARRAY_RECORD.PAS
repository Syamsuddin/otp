{ SPDX-License-Identifier: GPL-3.0-only }
program DataSiswa;
type TSiswa = record
  nama: string[30];
  nilai: Integer;
end;
var siswa: array[1..3] of TSiswa; i, total: Integer;
begin
  siswa[1].nama := 'Ayu'; siswa[1].nilai := 90;
  siswa[2].nama := 'Budi'; siswa[2].nilai := 80;
  siswa[3].nama := 'Citra'; siswa[3].nilai := 85;
  total := 0;
  for i := 1 to 3 do begin
    WriteLn(siswa[i].nama:10, siswa[i].nilai:5);
    total := total + siswa[i].nilai;
  end;
  WriteLn('Rata-rata: ', total / 3:0:2);
end.
