/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   
   Naive transliteration from Michael Ferguson's Chapel program   
   contributed by Isaac Gouy  
*/

package main

import (
   "flag"
   "fmt"
   "math"
   "strconv"
)

const (
   solarMass = 4 * math.Pi * math.Pi
   daysPerYear = 365.24
)

type Body struct {
   x, y, z, vx, vy, vz, mass float64
}

func offsetMomentum(bodies []Body) {
   px, py, pz := 0.0, 0.0, 0.0
   for i := range bodies {
      px += bodies[i].vx * bodies[i].mass
      py += bodies[i].vy * bodies[i].mass
      pz += bodies[i].vz * bodies[i].mass
    } 
   bodies[0].vx = -px / solarMass
   bodies[0].vy = -py / solarMass
   bodies[0].vz = -pz / solarMass 
}

func energy(bodies []Body) float64 {
   var e float64
   numBodies := len(bodies)  
   for i := 0; i < numBodies; i++ {   
      b := bodies[i]     
      sq := b.vx * b.vx + b.vy * b.vy + b.vz * b.vz      
      e += 0.5 * b.mass * sq
      for j := i + 1; j < numBodies; j++ {
         dx := b.x - bodies[j].x
         dy := b.y - bodies[j].y
         dz := b.z - bodies[j].z
         dsq := dx * dx + dy * dy + dz * dz         
         e -= (b.mass * bodies[j].mass) / math.Sqrt(dsq)         
      }
   }
   return e
}

func advance(bodies []Body, dt float64) {
   numBodies := len(bodies)  
   for i := 0; i < numBodies; i++ {  
      for j := i + 1; j < numBodies; j++ {   
         dx := bodies[i].x - bodies[j].x
         dy := bodies[i].y - bodies[j].y
         dz := bodies[i].z - bodies[j].z			
         dsq := dx*dx + dy*dy + dz*dz				   
         mag := dt / (dsq * math.Sqrt(dsq))

         mj := bodies[j].mass * mag
         bodies[i].vx -= dx * mj;
         bodies[i].vy -= dy * mj;
         bodies[i].vz -= dz * mj;

         mi := bodies[i].mass * mag
         bodies[j].vx += dx * mi
         bodies[j].vy += dy * mi
         bodies[j].vz += dz * mi
      }
   }

   for i := 0; i < numBodies; i++ { 
      bodies[i].x += dt * bodies[i].vx
      bodies[i].y += dt * bodies[i].vy
      bodies[i].z += dt * bodies[i].vz
   }
}

func nbody(n int) {
   bodies := []Body { 
      // sun    
      Body {
         mass: solarMass,
      },   
      // jupiter      
      Body {
         x: 4.84143144246472090e+00,
         y: -1.16032004402742839e+00,
         z: -1.03622044471123109e-01,
         vx: 1.66007664274403694e-03 * daysPerYear,
         vy: 7.69901118419740425e-03 * daysPerYear,
         vz: -6.90460016972063023e-05 * daysPerYear,
         mass: 9.54791938424326609e-04 * solarMass,
      },
      // saturn      
      Body {
         x: 8.34336671824457987e+00,
         y: 4.12479856412430479e+00,
         z: -4.03523417114321381e-01,
         vx: -2.76742510726862411e-03 * daysPerYear,
         vy: 4.99852801234917238e-03 * daysPerYear,
         vz: 2.30417297573763929e-05 * daysPerYear,
         mass: 2.85885980666130812e-04 * solarMass,
      },     
      // uranus      
      Body {
         x: 1.28943695621391310e+01,
         y: -1.51111514016986312e+01,
         z: -2.23307578892655734e-01,
         vx: 2.96460137564761618e-03 * daysPerYear,
         vy: 2.37847173959480950e-03 * daysPerYear,
         vz: -2.96589568540237556e-05 * daysPerYear,
         mass: 4.36624404335156298e-05 * solarMass,
      },      
      // neptune      
      Body {      
         x: 1.53796971148509165e+01,
         y: -2.59193146099879641e+01,
         z: 1.79258772950371181e-01,
         vx: 2.68067772490389322e-03 * daysPerYear,
         vy: 1.62824170038242295e-03 * daysPerYear,
         vz: -9.51592254519715870e-05 * daysPerYear,
         mass: 5.15138902046611451e-05 * solarMass,      
      },      
   }

   offsetMomentum(bodies)  
   fmt.Printf("%.9f\n", energy(bodies))
   for i := 0; i < n; i++ {
      advance(bodies, 0.01)
   }
   fmt.Printf("%.9f\n", energy(bodies))    
}

func main() {
   n := 0
   flag.Parse()
   if flag.NArg() > 0 { n,_ = strconv.Atoi( flag.Arg(0) ) }
   nbody(n)
}
