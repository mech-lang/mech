/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   
   Naive transliteration from Michael Ferguson's Chapel program
   contributed by Isaac Gouy   
*/

import 'dart:math' as Math;

const SOLAR_MASS = 4 * Math.pi * Math.pi;
const DAYS_PER_YEAR = 365.24;

class Body {
  double x, y, z, vx, vy, vz, mass;
   
  Body(this.x, this.y, this.z, this.vx, this.vy, this.vz, this.mass);
}

void offsetMomentum(List<Body> bodies) {
  var px = 0.0, py = 0.0, pz = 0.0;
  for (final b in bodies) {  
    final mass = b.mass;
    px += b.vx * mass;
    py += b.vy * mass;
    pz += b.vz * mass;  
  }  
  final b = bodies[0];
  b.vx = -px / SOLAR_MASS;
  b.vy = -py / SOLAR_MASS;
  b.vz = -pz / SOLAR_MASS;
}

double energy(List<Body> bodies) {
  var e = 0.0;
  final numBodies = bodies.length;
  for (var i = 0; i < numBodies; i++) {
    final b = bodies[i];
    final sq = b.vx * b.vx + b.vy * b.vy + b.vz * b.vz; 
    e += 0.5 * b.mass * sq;
    for (var j = i + 1; j < numBodies; j++) {
      final bodyj = bodies[j];
      final dx = b.x - bodyj.x;
      final dy = b.y - bodyj.y;
      final dz = b.z - bodyj.z;
      final sq = dx * dx + dy * dy + dz * dz; 
      final distance = Math.sqrt(dx * dx + dy * dy + dz * dz);
      e -= (b.mass * bodyj.mass) / Math.sqrt(sq);
    }
  }
  return e;
}

void advance(List<Body> bodies, double dt) {
  final numBodies = bodies.length;
  for (var i = 0; i < numBodies; i++) {
    for (var j = i + 1; j < numBodies; j++) {
      final dx = bodies[i].x - bodies[j].x; 
      final dy = bodies[i].y - bodies[j].y; 
      final dz = bodies[i].z - bodies[j].z;          
      final sq = dx * dx + dy * dy + dz * dz;
      final mag = dt / (sq * Math.sqrt(sq));

      final mj = bodies[j].mass * mag;
      bodies[i].vx -= dx * mj;
      bodies[i].vy -= dy * mj;
      bodies[i].vz -= dz * mj;
            
      final mi = bodies[i].mass * mag;            
      bodies[j].vx += dx * mi;
      bodies[j].vy += dy * mi;
      bodies[j].vz += dz * mi;
    }
  }
  
  for (final b in bodies) {    
    b.x += b.vx * dt;  
    b.y += b.vy * dt;   
    b.z += b.vz * dt;
  }
}


void main(List<String> args) {
  final n = (args.length > 0) ? int.parse(args[0]) : 1000;
  
  final bodies = [
    // sun    
    Body(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, SOLAR_MASS),
    
    // jupiter    
    Body(    
      4.84143144246472090e+00,
      -1.16032004402742839e+00,
      -1.03622044471123109e-01,
      1.66007664274403694e-03 * DAYS_PER_YEAR,
      7.69901118419740425e-03 * DAYS_PER_YEAR,
      -6.90460016972063023e-05 * DAYS_PER_YEAR,
      9.54791938424326609e-04 * SOLAR_MASS
    ),    
    
    // saturn    
    Body(     
      8.34336671824457987e+00,
      4.12479856412430479e+00,
      -4.03523417114321381e-01,
      -2.76742510726862411e-03 * DAYS_PER_YEAR,
      4.99852801234917238e-03 * DAYS_PER_YEAR,
      2.30417297573763929e-05 * DAYS_PER_YEAR,
      2.85885980666130812e-04 * SOLAR_MASS
    ),
      
    // uranus    
    Body(       
      1.28943695621391310e+01,
      -1.51111514016986312e+01,
      -2.23307578892655734e-01,
      2.96460137564761618e-03 * DAYS_PER_YEAR,
      2.37847173959480950e-03 * DAYS_PER_YEAR,
      -2.96589568540237556e-05 * DAYS_PER_YEAR,
      4.36624404335156298e-05 * SOLAR_MASS
    ), 
    
    // neptune    
    Body(       
      1.53796971148509165e+01,
      -2.59193146099879641e+01,
      1.79258772950371181e-01,
      2.68067772490389322e-03 * DAYS_PER_YEAR,
      1.62824170038242295e-03 * DAYS_PER_YEAR,
      -9.51592254519715870e-05 * DAYS_PER_YEAR,
      5.15138902046611451e-05 * SOLAR_MASS
    ),     
  ];  
  
  offsetMomentum(bodies);
  print(energy(bodies).toStringAsFixed(9));
  for (var i = 0; i < n; i++) {
    advance(bodies,0.01);
  }  
  print(energy(bodies).toStringAsFixed(9));  
}
