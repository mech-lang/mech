{ The Computer Language Benchmarks Game
  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

  contributed by Ales Katona
  modified by Vincent Snijders
  optimized and multithreaded by Jean de La Taille
  fixed by Peter Blackman
}

program mandelbrot;

uses
  {$ifdef unix}cthreads,{$endif}
  sysUtils, dateUtils, math;

const
  Limit = 4;

var
  n, n_1, dimx, dimy : longint;
  TextBuf: array of byte;
  start, finish : TDateTime;

type
  mem = record
    from_y, to_y : longint;
  end;
  pmem = ^mem;

function subThread(p: pointer) : ptrint;
var
  x, y, from_y, to_y, buf_index, i: Longint;
  Zr, Zi, Ti, Tr : Double;
  Cr, Ci : Double;
  bits: Longint;
  bit: Longint;
begin
  from_y := pmem(p)^.from_y;
  to_y := pmem(p)^.to_y;
  buf_index := from_y * dimx;
  for y := from_y to to_y do
  begin
    bit := 128; // 1000 0000
    bits := 0;
    Ci := ((y + y) / n) - 1.0;
    for x := 0 to n_1 do
    begin
      //---------------------------
      Zr := 0;
      Zi := 0;
      Tr := 0;
      Ti := 0;
      Cr := ((x + x) / n) - 1.5;
      for i := 1 to 50 do
      begin
        Zi := 2 * Zr * Zi + Ci;
        Zr := Tr - Ti + Cr;
        Ti := Zi * Zi;
        Tr := Zr * Zr;
        if ((Tr + Ti) > limit) then
        begin
          bits := bits or bit;
          break;
        end;
      end;
      //---------------------------
      bit := bit >> 1;
      if (bit = 0) then
      begin
        TextBuf[buf_index] := not bits;
        inc(buf_index);
        bits := 0;
        bit := 128;
      end;
    end;
  end;
  subThread := 0;
end;

procedure run;
var
  i, l, x, y, buf_index: Longint;
  tt : array[0..3] of TThreadID;
  m : array[0..3] of mem;
  stepL : longint;
begin
  n_1 := n - 1;
  l := 0;
  stepL := floor(n / 4);

  start := now;
  for i := 0 to 2 do
  begin
    m[i].from_y := l;
    m[i].to_y := l + stepL - 1;
    tt[i] := BeginThread(@subThread, @m[i]);
    l := l + stepL;
  end;
  m[3].from_y := l;
  m[3].to_y := n_1;
  tt[3] := BeginThread(@subThread, @m[3]);
  for i := 0 to 3 do
    WaitForThreadTerminate(tt[i], 0);
  finish := now;
  //WriteLn('Time : ', MilliSecondsBetween(start, finish) / 1000 : 0 : 4);

  buf_index := 0;
  for y := 0 to n_1 do
    for x := 0 to dimx - 1 do
    begin
      write(chr(TextBuf[buf_index]));;
      inc(buf_index);
    end;
end;


begin
  Val(ParamStr(1), n);
  write('P4', chr(10), n, ' ', n, chr(10));
  //write('P5', chr(10), n, ' ', n, chr(10), 255, chr(10));

  dimx := Ceil(n / 8);
  dimy := n;
  SetLength(TextBuf, (dimx * dimy) + 1);

  start := now;

  run;

  finish := now;
end.
