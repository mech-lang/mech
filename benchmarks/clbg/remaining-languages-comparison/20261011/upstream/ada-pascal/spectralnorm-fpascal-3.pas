{ The Computer Language Benchmarks Game
  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

  - contributed by Ian Osgood
  - modified by Vincent Snijders
  - modified by Peter Blackman
  - modified by Akira1364
}

program spectralnorm;

uses cmem, {$ifdef UNIX}cthreads,{$endif} mtprocs;

type
  aod = array of double;
  paod = ^aod;
  aodpair = array[0..1] of paod;

var
  i, n, len: ptrint;
  u, v, tmp: aod;
  vBv, vv: double;
  up, uplow, vp: pdouble;
  w: aodpair;
  wp: ^aodpair;

function A(const i,j : ptrint): double; inline;
begin
    A := 1 / ((i + j) * (i + j + 1) div 2 + i + 1);
end;

procedure mulAv(i: ptrint; Data: Pointer; Item: TMultiThreadProcItem);
var
  j: ptrint;
  q: double;
begin
  q := 0;
  for j := 0 to n - 1 do
    q := q + A(i,j) * aodpair(Data^)[0]^[j];
  aodpair(Data^)[1]^[i] := q;
end;

procedure mulAtv(i: ptrint; Data: Pointer; Item: TMultiThreadProcItem);
var
  j: ptrint;
  q: double;
begin
  q := 0;
  for j := 0 to n - 1 do
    q := q + A(j,i) * aodpair(Data^)[0]^[j];
  aodpair(Data^)[1]^[i] := q;
end;

procedure mulAtAv(const PAtA1, PAtA2: paod); inline;
begin
  w[0] := PAtA1;
  w[1] := @tmp;
  ProcThreadPool.DoParallel(@mulAv, 0, len, wp);
  w[0] := @tmp;
  w[1] := PAtA2;
  ProcThreadPool.DoParallel(@mulAtv, 0, len, wp);
end;

begin
  val(paramstr(1), n, i);
  setlength(u, n);
  setlength(v, n);
  setlength(tmp, n);

  len := n - 1;

  for i := len downto 0 do
    u[i] := 1.0;

  vBv := 0;
  vv := 0;
  wp := @w;

  for i := 1 to 10 do begin
    mulAtAv(@u, @v);
    mulAtAv(@v, @u);
  end;

  up := @u[len];
  uplow := @u[0];
  vp := @v[len];
  repeat
    vBv := vBv + up^ * vp^;
    vv := vv + vp^ * vp^;
    Dec(up);
    Dec(vp);
  until up = uplow;

  writeln(sqrt(vBv / vv) : 0 : 9);
end.
