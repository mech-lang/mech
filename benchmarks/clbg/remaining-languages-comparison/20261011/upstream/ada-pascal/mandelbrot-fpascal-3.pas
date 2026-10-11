{ The Computer Language Benchmarks Game
  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

  contributed by Ales Katona
  modified by Vincent Snijders
}

program mandelbrot;

var n: longint;
    TextBuf: array[0..$FFF] of byte;
    OutFile: PText;
    

procedure run;
var
  Cy, Step: double;
  x, y, bits,bit: Longint;
  function CalculatePoint(Cx, Cy: double): boolean; inline;
  const
    Limit = 4;
  var
    i: longint;
    Zr, Zi, Ti, Tr: Double;

  begin
    Zr := 0;  Zi := 0; Tr := 0; Ti := 0;
    for i := 1 to 50 do begin
      Zi := 2*Zr*Zi + Cy;
      Zr := Tr - Ti + Cx;
      Ti := Zi * Zi;
      Tr := Zr * Zr;
      if (Tr + Ti>=limit) then exit(true);
    end;

    CalculatePoint := false;
  end;

begin
  Step := 2/n;
  for y := 0 to n-1 do
  begin
    Cy := y * Step - 1;
    bits := 255;  bit := 128;
    for x := 0 to n-1 do
    begin
      if CalculatePoint(x * Step  - 1.5, Cy) then
        bits := bits xor bit;

      if bit > 1 then
        bit := bit shr 1
      else
      begin
        write(OutFile^, chr(bits));
        bits := 255;  bit := 128;
      end;
    end;
    if bit < 128 then write(OutFile^, chr(bits xor((bit shl 1)-1)));
  end;
end;

begin
  OutFile := @Output;
  SetTextBuf(OutFile^, TextBuf);

  Val(ParamStr(1), n);
  writeln(OutFile^, 'P4');
  writeln(OutFile^, n,' ',n);
  run;
end.
