// Faithful Benchmarks Game workload; local Toit candidate.
/* The Computer Language Benchmarks Game
  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

  Naive transliteration from Michael Ferguson's Chapel program
  contributed by Isaac Gouy
*/

import math

SOLAR-MASS ::= 4 * math.PI * math.PI
DAYS-PER-YEAR ::= 365.24

class Body:
  x /float := ?
  y /float := ?
  z /float := ?
  vx /float := ?
  vy /float := ?
  vz /float := ?
  mass /float := ?
  
  constructor .x .y .z .vx .vy .vz .mass:       
  
offset-momentum bodies/List:
  px := 0.0
  py := 0.0
  pz := 0.0  
  bodies.do: 
    px += it.vx * it.mass
    py += it.vy * it.mass
    pz += it.vz * it.mass
    
  b := bodies[0]
  b.vx = -px / SOLAR-MASS
  b.vy = -py / SOLAR-MASS
  b.vz = -pz / SOLAR-MASS
  
energy bodies/List -> float:  
  e := 0.0  
  num-bodies := bodies.size   
  num-bodies.repeat:
    b := bodies[it]
    sq := b.vx * b.vx + b.vy * b.vy + b.vz * b.vz
    e += 0.5 * bodies[it].mass * sq   
    for j := it + 1; j < num-bodies; ++j: 
      dx := b.x - bodies[j].x 
      dy := b.y - bodies[j].y 
      dz := b.z - bodies[j].z   
      sq = dx * dx + dy * dy + dz * dz          
      e -= (b.mass * bodies[j].mass) / (math.sqrt sq)    
  return e  
  
advance bodies/List dt/float:
  n := bodies.size
  for i := 0; i < n; i++:
    a := bodies[i]
    for j := i + 1; j < n; j++:
      b := bodies[j]
      dx := a.x - b.x
      dy := a.y - b.y
      dz := a.z - b.z
      squared := dx * dx + dy * dy + dz * dz
      mag := dt / (squared * (math.sqrt squared))
      bm := b.mass * mag
      a.vx -= dx * bm
      a.vy -= dy * bm
      a.vz -= dz * bm
      am := a.mass * mag
      b.vx += dx * am
      b.vy += dy * am
      b.vz += dz * am
  for i := 0; i < n; i++:
    a := bodies[i]
    a.x += a.vx * dt
    a.y += a.vy * dt
    a.z += a.vz * dt

nbody n/int:  
  bodies := [
    // Sun
    Body 0.0 0.0 0.0 0.0 0.0 0.0 SOLAR-MASS,
      
    Body  
      // Jupiter      
      4.84143144246472090e+00
      -1.16032004402742839e+00
      -1.03622044471123109e-01
      1.66007664274403694e-03 * DAYS-PER-YEAR
      7.69901118419740425e-03 * DAYS-PER-YEAR
      -6.90460016972063023e-05 * DAYS-PER-YEAR
      9.54791938424326609e-04 * SOLAR-MASS,     
      
    Body  
      // Saturn 
      8.34336671824457987e+00
      4.12479856412430479e+00
      -4.03523417114321381e-01
      -2.76742510726862411e-03 * DAYS_PER-YEAR
      4.99852801234917238e-03 * DAYS-PER-YEAR
      2.30417297573763929e-05 * DAYS-PER-YEAR
      2.85885980666130812e-04 * SOLAR-MASS,      
      
    Body 
      // Uranus       
      1.28943695621391310e+01
      -1.51111514016986312e+01
      -2.23307578892655734e-01
      2.96460137564761618e-03 * DAYS-PER-YEAR
      2.37847173959480950e-03 * DAYS-PER-YEAR
      -2.96589568540237556e-05 * DAYS-PER-YEAR
      4.36624404335156298e-05 * SOLAR-MASS, 
        
    Body 
      // Neptune         
      1.53796971148509165e+01
      -2.59193146099879641e+01
      1.79258772950371181e-01
      2.68067772490389322e-03 * DAYS-PER-YEAR
      1.62824170038242295e-03 * DAYS-PER-YEAR
      -9.51592254519715870e-05 * DAYS-PER-YEAR
      5.15138902046611451e-05 * SOLAR-MASS,         
    ]

  offset-momentum bodies
  print "$(%.9f energy bodies)" 
  n.repeat:
    advance bodies 0.01
  print "$(%.9f energy bodies)"   

main args:
  n := args.size == 1 ? int.parse args[0] : 0  
  nbody n
