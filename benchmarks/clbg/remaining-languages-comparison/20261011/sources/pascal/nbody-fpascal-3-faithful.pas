{ The Computer Language Benchmarks Game
  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

  contributed by Ian Osgood,
  modified by Florian Klaempfl
  modified by Ales Katona
  modified by Vincent Snijders
  modified by Thierry Coq
}

{$mode objfpc}

program NBody_Submission_2;

uses Math, SysUtils;

type
  Body = record
    x, y, z,
    vx, vy, vz,
    mass : double;
  end;
  PBody = ^Body;

const pi = 3.141592653589793;
      solarMass = 4 * sqr(pi);
      daysPerYear = 365.24;
      lowB = 1;
      highB = 5;

type
  tbody = array[lowB..highB] of Body;

const b : tbody = (
  { Sun }
  ( x:0; y:0; z:0;  vx:0; vy:0; vz:0;  mass: solarMass ),
  { Jupiter }
  ( x:    4.84143144246472090e+00;
    y:   -1.16032004402742839e+00;
    z:   -1.03622044471123109e-01;
    vx:   1.66007664274403694e-03 * daysPerYear;
    vy:   7.69901118419740425e-03 * daysPerYear;
    vz:  -6.90460016972063023e-05 * daysPerYear;
    mass: 9.54791938424326609e-04 * solarMass ),
  { Saturn }
  ( x:    8.34336671824457987e+00;
    y:    4.12479856412430479e+00;
    z:   -4.03523417114321381e-01;
    vx:  -2.76742510726862411e-03 * daysPerYear;
    vy:   4.99852801234917238e-03 * daysPerYear;
    vz:   2.30417297573763929e-05 * daysPerYear;
    mass: 2.85885980666130812e-04 * solarMass ),
  { Uranus }
  ( x:    1.28943695621391310e+01;
    y:   -1.51111514016986312e+01;
    z:   -2.23307578892655734e-01;
    vx:   2.96460137564761618e-03 * daysPerYear;
    vy:   2.37847173959480950e-03 * daysPerYear;
    vz:  -2.96589568540237556e-05 * daysPerYear;
    mass: 4.36624404335156298e-05 * solarMass ),
  { Neptune }
  ( x:    1.53796971148509165e+01;
    y:   -2.59193146099879641e+01;
    z:    1.79258772950371181e-01;
    vx:   2.68067772490389322e-03 * daysPerYear;
    vy:   1.62824170038242295e-03 * daysPerYear;
    vz:  -9.51592254519715870e-05 * daysPerYear;
    mass: 5.15138902046611451e-05 * solarMass )
);

procedure offsetMomentum;
var px,py,pz : double;
    i : integer;
begin
  px:=0.0; py:=0.0; pz:=0.0;
  for i := lowB+1 to highB do
    with b[i] do
    begin
      px := px - vx * mass;
      py := py - vy * mass;
      pz := pz - vz * mass;
    end;
  b[lowB].vx := px / solarMass;
  b[lowB].vy := py / solarMass;
  b[lowB].vz := pz / solarMass;
end;

function distance(const i,j : integer) : double;
begin
  distance := sqrt(sqr(b[i].x-b[j].x) + sqr(b[i].y-b[j].y) +
sqr(b[i].z-b[j].z));
end;

function energy : double;
var
  i,j : integer;
begin
  result := 0.0;
  for i := low(b) to high(b) do
    with b[i] do
    begin
      result := result + mass * (sqr(vx) + sqr(vy) + sqr(vz)) / 2;
      for j := i+1 to high(b) do
        result := result - mass * b[j].mass / distance(i,j);
    end;
end;

procedure advance(const dt : double);
var i,j : shortint;
    dx,dy,dz,mag, jmag, imag : double;
    bi,bj : PBody;
begin
  bi:=@b[low(b)];
  for i := lowB to highB-1 do begin
    bj := bi;
    with bi^ do
    begin
      for j := i+1 to highB do
      begin
        inc(bj);
        dx := x - bj^.x;
        dy := y - bj^.y;
        dz := z - bj^.z;

        mag := dt / (sqrt(dx*dx+dy*dy+dz*dz)*(dx*dx+dy*dy+dz*dz));
        jmag := bj^.mass * mag;
        imag := mass * mag;

        vx := vx - dx * jmag;
        vy := vy - dy * jmag;
        vz := vz - dz * jmag;
        bj^.vx := bj^.vx + dx * imag;
        bj^.vy := bj^.vy + dy * imag;
        bj^.vz := bj^.vz + dz * imag;
      end;
      x := x + dt * vx;
      y := y + dt * vy;
      z := z + dt * vz;
    end;
    inc(bi);
  end;
  //last one.
  bi^.x := bi^.x + dt * bi^.vx;
  bi^.y := bi^.y + dt * bi^.vy;
  bi^.z := bi^.z + dt * bi^.vz;
end;

var i : integer;
    n : Integer;
    start_time, end_time : TDateTime;
begin
  start_time := now;

  offsetMomentum;
  writeln(energy:0:9);
  Val(ParamStr(1), n, i);
  for i := 1 to n do advance(0.01);
  writeln(energy:0:9);
  end_time := now;
  //writeln('start time:', start_time);
  //writeln('end time  :', end_time);
  //writeln('delay     :', end_time-start_time);
  //writeln('delay (s) :', (end_time-start_time)*24*3600);
  //readln;
end.
