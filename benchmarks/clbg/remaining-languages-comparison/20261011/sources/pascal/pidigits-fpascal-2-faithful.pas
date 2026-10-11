{ The Computer Language Benchmarks Game
  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

  contributed by Vincent Snijders
  gmp headers by Karl-Michael Schindler
}

{$linklib gmp}
{$mode objfpc}

program pidigits;

uses
  ctypes;
  
type

mp_limb_t        = culong;
mp_ptr           = ^mp_limb_t;

mpz_t = record
  mp_alloc, mp_size : cint;
  mp_d    : mp_ptr
end;

procedure mpz_init (var Dest: mpz_t);
 cdecl; external name '__gmpz_init';

procedure mpz_init_set_ui (var Dest: mpz_t; Src: culong);
 cdecl; external name '__gmpz_init_set_ui';

procedure mpz_mul_ui (var Dest: mpz_t; var Src1: mpz_t; Src2: culong);
 cdecl; external name '__gmpz_mul_ui';

procedure mpz_mul_si (var Dest: mpz_t; var Src1: mpz_t; Src2: cint);
 cdecl; external name '__gmpz_mul_si';

procedure mpz_add (var Dest: mpz_t; var Src1, Src2: mpz_t);
 cdecl; external name '__gmpz_add';

procedure mpz_tdiv_q (var Dest: mpz_t; var Src1, Src2: mpz_t);
 cdecl; external name '__gmpz_tdiv_q';

function  mpz_get_ui (var Src: mpz_t): culong;
 cdecl; external name '__gmpz_get_ui';

  
procedure PrintPiDigits(NumDigits: integer);
var
  q, r, s, t: mpz_t; // Transformation matrix components.
  u, v, w: mpz_t; // Temporary variables
  i, k, digit, c, padding: integer;
  line: string[10];
  
  function Extract(x:cardinal) : integer;
  begin
    mpz_mul_ui(u, q, x);
    mpz_add(u, u, r);
    mpz_mul_ui(v, s, x);
    mpz_add(v, v, t);
    mpz_tdiv_q(w, u, v);
    result := mpz_get_ui(w);
  end;

  function IsSafe : boolean;
  begin
    result := digit = Extract(4);
  end;

  procedure Produce;
  begin
    mpz_mul_si(r, r, 10);
    mpz_mul_si(v, t, -10 * digit);
    mpz_add(r, r, v);
    mpz_mul_si(q, q, 10);
  end;

  procedure Consume;
  begin
    inc(k);
    mpz_mul_si(r, r, 2*k+1);
    mpz_mul_si(u, q, 4*k+2);
    mpz_add(r, r, u);
    mpz_mul_si(t, t, 2*k+1);
    mpz_mul_si(v, s, 4*k+2);
    mpz_add(t, t, v);
    mpz_mul_si(s, s, k);
    mpz_mul_si(q, q, k);
  end;

begin
  k := 0;
  i := 0;
  c := 0;
  setlength(line, 10);
  mpz_init_set_ui(q, 1);
  mpz_init_set_ui(r, 0);
  mpz_init_set_ui(s, 0);
  mpz_init_set_ui(t, 1);
  mpz_init(u);
  mpz_init(v);
  mpz_init(w);
  while (i<NumDigits) do begin
    digit := Extract(3);
    while not IsSafe do begin
      Consume;
      digit:= Extract(3);
    end;
    Produce;
    inc(c);
    line[c] := chr(ord('0')+digit);
    inc(i);
    if c=10 then begin
      writeln(line, #9':', i);
      c := 0;
    end;
  end;
  if c<>0 then begin
    write(Copy(line,1,c));
    for padding := c+1 to 10 do write(' ');
    writeln(#9':',i);
  end;
end;

var
  n: integer = 27;

begin
  if (ParamCount=1) then
    val(ParamStr(1), n);
  PrintPiDigits(n);
end.
