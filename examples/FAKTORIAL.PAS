{ SPDX-License-Identifier: GPL-3.0-only }
program Faktorial;
function Fact(n: Integer): LongInt;
begin
  if n <= 1 then Fact := 1
  else Fact := n * Fact(n - 1);
end;
var i: Integer;
begin
  for i := 1 to 10 do
    WriteLn(i:2, '! = ', Fact(i):8);
end.
