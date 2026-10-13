{ SPDX-License-Identifier: GPL-3.0-only }
program Parameter;
var x: Integer;
procedure Tambah(var nilai: Integer; jumlah: Integer);
begin nilai := nilai + jumlah end;
procedure Luar;
var lokal: Integer;
  procedure Dalam;
  begin Inc(lokal); Tambah(x, lokal) end;
begin lokal := 4; Dalam end;
begin x := 10; Luar; WriteLn(x) end.
