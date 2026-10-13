{ SPDX-License-Identifier: GPL-3.0-only }
program Belajar;
var nama: string[40]; panjang, lebar: Real;
begin
  Write('Nama: '); ReadLn(nama);
  Write('Panjang: '); ReadLn(panjang);
  Write('Lebar: '); ReadLn(lebar);
  WriteLn('Halo ', nama);
  WriteLn('Luas = ', panjang * lebar:0:2);
end.
