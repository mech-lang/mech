/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   
   Naive transliteration from Michael Ferguson's Chapel program   
   contributed by Isaac Gouy 
*/

import Glibc

let PI = 3.141592653589793
let SOLAR_MASS = 4 * PI * PI
let DAYS_PER_YEAR = 365.24

class Body {
   var x, y, z, vx, vy, vz, mass : Double

   init(x: Double, y: Double, z: Double, 
         vx: Double, vy: Double, vz: Double, mass: Double) {
      self.x = x
      self.y = y
      self.z = z
      self.vx = vx
      self.vy = vy
      self.vz = vz
      self.mass = mass
   }
}

func offsetMomentum(_ bodies: [Body]) {
   var px = 0.0
   var py = 0.0
   var pz = 0.0
   for b in bodies {
      px += b.vx * b.mass
      py += b.vy * b.mass		
      pz += b.vz * b.mass	
   }
   let b = bodies[0]
   b.vx = -px / SOLAR_MASS
   b.vy = -py / SOLAR_MASS
   b.vz = -pz / SOLAR_MASS
}

func energy(_ bodies: [Body]) -> Double {
   var e = 0.0		   
   let numBodies = bodies.count		
   for i in 0..<numBodies {
      let b = bodies[i]   
      let sq = b.vx * b.vx + b.vy * b.vy + b.vz * b.vz;   
      e += 0.5 * bodies[i].mass * sq;			  
      for j in i+1..<numBodies {
         let dx = b.x - bodies[j].x
         let dy = b.y - bodies[j].y
         let dz = b.z - bodies[j].z
         let dsq = dx * dx + dy * dy + dz * dz;                                         
         e -= (b.mass * bodies[j].mass) / sqrt(dsq)
      }
   }
   return e;
}

func advance(_ bodies: [Body], _ dt: Double) {	
   let numBodies = bodies.count		
   for i in 0..<numBodies {
      for j in i+1..<numBodies {
         let dx = bodies[i].x - bodies[j].x
         let dy = bodies[i].y - bodies[j].y
         let dz = bodies[i].z - bodies[j].z			
         let dsq = dx*dx + dy*dy + dz*dz				   
         let mag = dt / (dsq * sqrt(dsq))

         let mj = bodies[j].mass * mag
         bodies[i].vx -= dx * mj
         bodies[i].vy -= dy * mj
         bodies[i].vz -= dz * mj

         let mi = bodies[i].mass * mag
         bodies[j].vx += dx * mi
         bodies[j].vy += dy * mi
         bodies[j].vz += dz * mi
      }
   }		

   for i in 0..<numBodies {
      bodies[i].x += bodies[i].vx * dt
      bodies[i].y += bodies[i].vy * dt
      bodies[i].z += bodies[i].vz * dt
   }	
}

func main(_ n: Int) {
   let bodies: [Body] = [
      // sun   
      Body (
         x: 0.0,
         y: 0.0,
         z: 0.0,
         vx: 0.0,
         vy: 0.0,
         vz: 0.0,
         mass: SOLAR_MASS
      ), 
      // jupiter
      Body (
         x: 4.84143144246472090e+00,
         y: -1.16032004402742839e+00,
         z: -1.03622044471123109e-01,
         vx: 1.66007664274403694e-03 * DAYS_PER_YEAR,
         vy: 7.69901118419740425e-03 * DAYS_PER_YEAR,
         vz: -6.90460016972063023e-05 * DAYS_PER_YEAR,
         mass: 9.54791938424326609e-04 * SOLAR_MASS
      ),
      // saturn
      Body (
         x: 8.34336671824457987e+00,
         y: 4.12479856412430479e+00,
         z: -4.03523417114321381e-01,
         vx: -2.76742510726862411e-03 * DAYS_PER_YEAR,
         vy: 4.99852801234917238e-03 * DAYS_PER_YEAR,
         vz: 2.30417297573763929e-05 * DAYS_PER_YEAR,
         mass: 2.85885980666130812e-04 * SOLAR_MASS
      ),
      // uranus 
      Body (
         x: 1.28943695621391310e+01,
         y: -1.51111514016986312e+01,
         z: -2.23307578892655734e-01,
         vx: 2.96460137564761618e-03 * DAYS_PER_YEAR,
         vy: 2.37847173959480950e-03 * DAYS_PER_YEAR,
         vz: -2.96589568540237556e-05 * DAYS_PER_YEAR,
         mass: 4.36624404335156298e-05 * SOLAR_MASS
      ),
      // neptune 
      Body (
         x: 1.53796971148509165e+01,
         y: -2.59193146099879641e+01,
         z: 1.79258772950371181e-01,
         vx: 2.68067772490389322e-03 * DAYS_PER_YEAR,
         vy: 1.62824170038242295e-03 * DAYS_PER_YEAR,
         vz: -9.51592254519715870e-05 * DAYS_PER_YEAR,
         mass: 5.15138902046611451e-05 * SOLAR_MASS
      )
   ]

   offsetMomentum(bodies)
   print(energy(bodies))
   for _ in 1...n {
      advance(bodies, 0.01)
   }
   print(energy(bodies))
}

main( 
   (CommandLine.argc > 1) 
      ? Int(CommandLine.arguments[1])! 
      : 1000 )
