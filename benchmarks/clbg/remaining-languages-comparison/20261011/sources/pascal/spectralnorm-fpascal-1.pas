{ The Computer Language Benchmarks Game
  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

  contributed by Ian Osgood
  modified by Vincent Snijders
}

program spectralNorm;


var n,i : integer;
    u,v,tmp : array of double;
    vBv,vv : double;

function A(i,j : integer): double; inline;
begin
  A := 1 / ((i+j)*(i+j+1) div 2 + i+1);
end;

procedure mulAv(var v, Av : array of double);
var i,j : integer;
begin
  for i := low(Av) to high(Av) do
  begin
    Av[i] := 0.0;
    for j := low(v) to high(v) do
      Av[i] := Av[i] + A(i,j) * v[j];
  end;
end;

procedure mulAtv(var v, Atv : array of double);
var i,j : integer;
begin
  for i := low(Atv) to high(Atv) do
  begin
    Atv[i] := 0.0;
    for j := low(v) to high(v) do
      Atv[i] := Atv[i] + A(j,i) * v[j];
  end;
end;

procedure mulAtAv(var v, AtAv : array of double);
begin
  mulAv(v, tmp);
  mulAtv(tmp, AtAv);
end;

begin
  Val(paramstr(1), n, i);
  SetLength(u, n);
  SetLength(v, n);
  SetLength(tmp, n);

  for i := low(u) to high(u) do u[i] := 1.0;

  for i := 1 to 10 do begin mulAtAv(u,v); mulAtAv(v,u) end;

  for i := low(u) to high(u) do
  begin
    vBv := vBv + u[i]*v[i];
    vv  := vv  + v[i]*v[i];
  end;

  writeln(sqrt(vBv/vv):0:9);
end.
