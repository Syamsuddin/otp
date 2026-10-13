#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Black-box conformance/regression checks. Compiles and executes native Pascal.
Requires: cargo build --workspace, Python 3, native cc/clang. No PyPI packages.
Usage: python3 tests/native_suite.py [--release]
"""
from pathlib import Path
import subprocess, tempfile, sys, os, socket, json, platform, time
ROOT=Path(__file__).resolve().parents[1]
PROFILE='release' if '--release' in sys.argv else 'debug'
CLI=ROOT/'target'/PROFILE/('otpc.exe' if os.name=='nt' else 'otpc')
CASES=[
 ('hello', "begin writeln('Hello Odin') end.", 'Hello Odin\n'),
 ('case_insensitive', "PROGRAM test; VAR X: INTEGER; BEGIN x:=42; WRITELN(X) END.", '42\n'),
 ('integers', "var b:byte; s:shortint; w:word; n:integer; l:longint; begin b:=255;s:=-128;w:=65535;n:=-32768;l:=2147483647;writeln(b,' ',s,' ',w,' ',n,' ',l) end.", '255 -128 65535 -32768 2147483647\n'),
 ('loop_control', "var i,s:integer; begin s:=0;for i:=1 to 10 do begin if i=3 then continue;if i=8 then break;s:=s+i end;while s<30 do inc(s);repeat dec(s) until s=28;writeln(s);for i:=3 downto 1 do write(i);writeln end.", '28\n321\n'),
 ('case', "var n:integer;begin for n:=0 to 4 do case n of 0,1:write('a');2..3:write('b');else write('c') end;writeln end.", 'aabbc\n'),
 ('short_circuit', "var i:integer;begin i:=0;if (i<>0) and (10 div i>0) then writeln('bad');if true or (1 div i=0) then writeln('ok') end.", 'ok\n'),
 ('nested_recursion', "var v:longint;function f(n:integer):longint;begin if n=0 then f:=1 else f:=n*f(n-1) end;procedure a;var x:integer;procedure b;begin inc(x);v:=v+x end;begin x:=4;b end;begin v:=f(10);a;writeln(v) end.", '3628805\n'),
 ('value_and_var', "type T=array[1..2]of integer;var a:T;procedure p(x:T;var y:integer);begin x[1]:=99;y:=7 end;begin a[1]:=4;p(a,a[2]);writeln(a[1],' ',a[2]) end.", '4 7\n'),
 ('var_alias', "type T=array[1..2]of integer;var a:T;procedure p(var x:T;var y:integer);begin x[2]:=9;y:=7 end;begin a[2]:=1;p(a,a[2]);writeln(a[2]) end.", '7\n'),
 ('multidimensional', "var a:array[-1..1,2..3]of integer;begin a[-1,3]:=42;writeln(a[-1][3]) end.", '42\n'),
 ('records', "type t=record x:integer;s:string[3] end;var a,b:t;begin a.x:=9;a.s:='abcdef';b:=a;b.x:=4;writeln(a.x,' ',b.x,' ',b.s) end.", '9 4 abc\n'),
 ('strings', "var s:string[5];c:char;begin s:='Pascal';c:='a';writeln(s,' ',length(s),' ',ord(c),' ',upcase(c));writeln(copy('Pascal',2,3),' ',pos('sc','Pascal'),' ',concat('O','DIN'));writeln(ord(s[0]),' ',s[1]) end.", 'Pasca 5 97 A\nasc 3 ODIN\n5 P\n'),
 ('real48', "var r:real;begin r:=1.25;writeln(r*4:0:2,' ',sqrt(9):0:1,' ',round(2.6),' ',trunc(-2.9)) end.", '5.00 3.0 3 -2\n'),
 ('wrap', "var n:integer;b:byte;begin n:=32767;n:=n+1;b:=256;writeln(n,' ',b) end.", '-32768 0\n'),
 ('constant_scope', "const a=3;b=a+4;procedure p;const a=20;begin writeln(b) end;begin p end.", '7\n'),
 ('literal_directive', "begin writeln('{$X+}') end.", '{$X+}\n'),
 ('long_expression', 'begin writeln('+ '+'.join(['1']*60)+') end.', '60\n'),
 ('char_case', "var c:char;begin c:='b';case c of 'a'..'c':writeln('ok');else writeln('bad') end end.", 'ok\n'),
 ('crt_coordinates', "uses crt;begin gotoxy(3,4);textcolor(12);writeln(wherex,' ',wherey) end.", '\x1b[4;3H\x1b[91m3 4\n'),
]
BAD=[
 ('undeclared', 'begin x:=1 end.', 'Variable expected'),
 ('mismatch', "var x:integer;begin x:='oops' end.", 'mismatch'),
 ('bool_condition', 'begin if 3 then writeln(1) end.', 'Boolean'),
 ('var_type', 'procedure p(var x:integer);begin end;var x:longint;begin p(x) end.', 'mismatch'),
 ('unsupported_unit', 'uses graph;begin end.', 'not implemented'),
 ('unsupported_pointer', 'type p=^integer;begin end.', 'Unsupported character'),
 ('circular_const', 'const a=b;b=a;begin writeln(a) end.', 'Circular'),
 ('bad_const', 'const a=x;var x:integer;begin end.', 'Constant'),
 ('local_directive', 'program p;{$R+}begin end.', 'Local compiler'),
 ('unknown_directive', '{$Z+}begin end.', 'not implemented'),
 ('paren_directive', '(*$Z+*)begin end.', 'not implemented'),
 ('case_overlap', 'begin case 1 of 1,1: writeln(1) end end.', 'Overlapping'),
 ('builtin_types', "begin writeln(copy('a','b',1)) end.", 'type mismatch'),
 ('write_width', "begin writeln(1:'x') end.", 'Integer field width'),
 ('float_bitwise', 'begin writeln(1.5 and 3) end.', 'Integer operands'),
]
ERRORS=[
 ('zero', 'var n:integer;begin n:=0;writeln(1 div n) end.',200),
 ('range', '{$R+}var b:byte;begin b:=256 end.',201),
 ('overflow', '{$Q+}var n:integer;begin n:=32767;n:=n+1 end.',215),
 ('bounds', 'var a:array[1..2]of integer;begin writeln(a[3]) end.',201),
]
report=[]
def run(args,**kw):return subprocess.run([str(a) for a in args],capture_output=True,text=True,timeout=20,**kw)
def record(name,ok,detail=''):
 report.append({'test':name,'passed':bool(ok),'detail':detail})
 print(('PASS ' if ok else 'FAIL ')+name+((': '+detail) if not ok else ''))
with tempfile.TemporaryDirectory(prefix='otp-native-') as tmp:
 d=Path(tmp)
 def build(name,source,debug=False):
  p=d/(name+'.pas');p.write_text(source);exe=d/(name+('.exe' if os.name=='nt' else ''));r=run([CLI,'build',p,'-o',exe]+(['--debug'] if debug else []));return p,exe,r
 for name,src,want in CASES:
  p,exe,b=build(name,src)
  if b.returncode:record(name,False,b.stderr);continue
  r=run([exe],cwd=d);record(name,r.returncode==0 and r.stdout==want,repr((r.returncode,r.stdout,r.stderr)))
 for name,src,want in BAD:
  p=d/(name+'.pas');p.write_text(src);r=run([CLI,'check',p]);record(name,r.returncode!=0 and want.lower() in r.stderr.lower(),r.stderr)
 for name,src,code in ERRORS:
  p,exe,b=build(name,src)
  if b.returncode:record(name,False,b.stderr);continue
  r=run([exe]);record(name,r.returncode==code and f'Runtime error {code}' in r.stderr,r.stderr)
 for name,stdin,want in [('PARAMETER','', '15\n'),('FILE_TEKS','', 'Belajar Pascal dengan ODIN\nFile teks lintas OS\n'),('BELAJAR','Ayu\n2.5\n4\n','Nama: Panjang: Lebar: Halo Ayu\nLuas = 10.00\n')]:
  p,exe,b=build('example_'+name,(ROOT/'examples'/(name+'.PAS')).read_text())
  if b.returncode:record(name,False,b.stderr);continue
  r=run([exe],input=stdin,cwd=d);record(name,r.returncode==0 and r.stdout==want,repr((r.stdout,r.stderr)))
 p,exe,b=build('args',"begin writeln(paramcount,' ',paramstr(1)) end.")
 if b.returncode:record('program_args',False,b.stderr)
 else:
  r=run([exe,'odin']);record('program_args',r.stdout=='1 odin\n',repr((r.stdout,r.stderr)))
 p,exe,b=build('debugger', 'var n:integer;\nbegin\n n:=1;\n n:=n+2;\n writeln(n);\nend.\n',True)
 if b.returncode:record('source_debugger',False,b.stderr)
 else:
  listener=socket.socket();listener.bind(('127.0.0.1',0));listener.listen();listener.settimeout(8)
  env=os.environ.copy();env['OTP_DEBUG_ADDR']='127.0.0.1:'+str(listener.getsockname()[1]);child=subprocess.Popen([str(exe)],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
  try:
   stream,_=listener.accept();stream.settimeout(8);reader=stream.makefile('rb');events=[reader.readline().decode()];stream.sendall(b'STEP\n');events.append(reader.readline().decode());stream.sendall(b'BREAK 5\nCONT\n');events.append(reader.readline().decode());stream.sendall(b'CONT\n');out,err=child.communicate(timeout=8)
   record('source_debugger',events[0].startswith('LINE 3 ') and 'n=1;' in events[1] and events[2].startswith('LINE 5 ') and 'n=3;' in events[2] and out=='3\n',repr((events,out,err)))
  finally:
   if child.poll() is None:child.kill();child.wait()
   listener.close()
 # Every backend must generate assembly; only this host's linker is exercised here.
 for target in ['linux-x64','windows-x64','macos-x64','macos-arm64']:
  p=d/'target.pas';p.write_text("var n:integer;begin n:=4;writeln(n) end.");dest=d/(target+'.s');r=run([CLI,'emit-asm',p,'--target',target,'-o',dest]);record('emit_'+target,r.returncode==0 and dest.exists(),r.stderr)
 # A failed compilation must leave a previously built output untouched.
 p,exe,b=build('preserve',"begin writeln('old') end.");before=exe.read_bytes();p.write_text('begin ?? end.');r=run([CLI,'build',p,'-o',exe]);record('preserve_good_executable',r.returncode!=0 and exe.read_bytes()==before)
result={'platform':platform.platform(),'profile':PROFILE,'passed':sum(x['passed'] for x in report),'total':len(report),'tests':report}
(ROOT/'target'/f'native-test-report-{PROFILE}.json').write_text(json.dumps(result,indent=2))
print(f"{result['passed']}/{result['total']} passed")
sys.exit(0 if result['passed']==result['total'] else 1)
