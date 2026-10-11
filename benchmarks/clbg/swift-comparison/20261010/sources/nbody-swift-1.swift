/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   contributed by Isaac Gouy 
*/

import Darwin

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

let PI = 3.141592653589793
let SOLAR_MASS = 4 * PI * PI
let DAYS_PER_YEAR = 365.24

var bodies: [Body] = [
   Body (
      x: 0.0,
      y: 0.0,
      z: 0.0,
      vx: 0.0,
      vy: 0.0,
      vz: 0.0,
      mass: SOLAR_MASS
   ), 

   Body (
      x: 4.84143144246472090e+00,
      y: -1.16032004402742839e+00,
      z: -1.03622044471123109e-01,
      vx: 1.66007664274403694e-03 * DAYS_PER_YEAR,
      vy: 7.69901118419740425e-03 * DAYS_PER_YEAR,
      vz: -6.90460016972063023e-05 * DAYS_PER_YEAR,
      mass: 9.54791938424326609e-04 * SOLAR_MASS
   ),

   Body (
      x: 8.34336671824457987e+00,
      y: 4.12479856412430479e+00,
      z: -4.03523417114321381e-01,
      vx: -2.76742510726862411e-03 * DAYS_PER_YEAR,
      vy: 4.99852801234917238e-03 * DAYS_PER_YEAR,
      vz: 2.30417297573763929e-05 * DAYS_PER_YEAR,
      mass: 2.85885980666130812e-04 * SOLAR_MASS
   ),

   Body (
      x: 1.28943695621391310e+01,
      y: -1.51111514016986312e+01,
      z: -2.23307578892655734e-01,
      vx: 2.96460137564761618e-03 * DAYS_PER_YEAR,
      vy: 2.37847173959480950e-03 * DAYS_PER_YEAR,
      vz: -2.96589568540237556e-05 * DAYS_PER_YEAR,
      mass: 4.36624404335156298e-05 * SOLAR_MASS
   ),

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


func offsetMomentum() {
   var px = 0.0
   var py = 0.0
   var pz = 0.0

   for body in bodies {
      px += body.vx * body.mass
      py += body.vy * body.mass		
      pz += body.vz * body.mass	
   }

   bodies[0].vx = -px / SOLAR_MASS

   bodies[0].vy = -py / SOLAR_MASS

   bodies[0].vz = -pz / SOLAR_MASS

}


func energy() -> Double {		
   var dx, dy, dz, distance: Double	
   var e = 0.0		   
		
   for i in 0..<bodies.count {
      e += 0.5 * bodies[i].mass * 
         ( bodies[i].vx * bodies[i].vx 
         + bodies[i].vy * bodies[i].vy 
         + bodies[i].vz * bodies[i].vz )
			   
      for j in i+1..<bodies.count {
         dx = bodies[i].x - bodies[j].x
         dy = bodies[i].y - bodies[j].y
         dz = bodies[i].z - bodies[j].z
                                
         distance = sqrt(dx*dx + dy*dy + dz*dz)
         e -= (bodies[i].mass * bodies[j].mass) / distance
      }
   }
   return e

}


func advance(_ dt: Double) {
   var dx, dy, dz, distance, mag: Double	
	
   for i in 0..<bodies.count {
      for j in i+1..<bodies.count {
         dx = bodies[i].x - bodies[j].x
         dy = bodies[i].y - bodies[j].y
         dz = bodies[i].z - bodies[j].z
				
         distance = sqrt(dx*dx + dy*dy + dz*dz)				   
         mag = dt / (distance * distance * distance)

         bodies[i].vx -= dx * bodies[j].mass * mag
         bodies[i].vy -= dy * bodies[j].mass * mag
         bodies[i].vz -= dz * bodies[j].mass * mag
                                
         bodies[j].vx += dx * bodies[i].mass * mag
         bodies[j].vy += dy * bodies[i].mass * mag
         bodies[j].vz += dz * bodies[i].mass * mag
      }
   }		

   for i in 0..<bodies.count {
      bodies[i].x += dt * bodies[i].vx
      bodies[i].y += dt * bodies[i].vy
      bodies[i].z += dt * bodies[i].vz
   }	
}


let n: Int = Int(CommandLine.arguments[1])!
offsetMomentum()
print( energy() )
for _ in 1...n {
   advance(0.01)
}
print( energy() )
