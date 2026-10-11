/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   contributed by Ralph Ganszky
*/

import Glibc

let PI = 3.141592653589793
let SOLAR_MASS = 4 * PI * PI
let DAYS_PER_YEAR = 365.24

var sun     = ( (0.0, 0.0, 0.0), (0.0, 0.0, 0.0), SOLAR_MASS )
var jupiter = ( (4.8414314424647209,
		-1.16032004402742839,
	        -0.103622044471123109),
                (1.66007664274403694e-03 * DAYS_PER_YEAR,
                 7.69901118419740425e-03 * DAYS_PER_YEAR,
                -6.90460016972063023e-05 * DAYS_PER_YEAR),
                 9.54791938424326609e-04 * SOLAR_MASS )
var saturn  = ( (8.34336671824457987,
		 4.12479856412430479,
		-4.03523417114321381e-01),
               (-2.76742510726862411e-03 * DAYS_PER_YEAR,
                 4.99852801234917238e-03 * DAYS_PER_YEAR,
                 2.30417297573763929e-05 * DAYS_PER_YEAR),
                 2.85885980666130812e-04 * SOLAR_MASS )
var uranus  = ( (1.28943695621391310e+01,
		-1.51111514016986312e+01,
		-2.23307578892655734e-01),
                (2.96460137564761618e-03 * DAYS_PER_YEAR,
                 2.37847173959480950e-03 * DAYS_PER_YEAR,
                -2.96589568540237556e-05 * DAYS_PER_YEAR),
                 4.36624404335156298e-05 * SOLAR_MASS)
var neptune = ( (1.53796971148509165e+01,
		-2.59193146099879641e+01,
		 1.79258772950371181e-01),
                (2.68067772490389322e-03 * DAYS_PER_YEAR,
                 1.62824170038242295e-03 * DAYS_PER_YEAR,
                -9.51592254519715870e-05 * DAYS_PER_YEAR),
                 5.15138902046611451e-05 * SOLAR_MASS)

var bodies = [sun, jupiter, saturn, uranus, neptune]

func advance(_ bodies: inout [((Double,Double,Double),
			     (Double,Double,Double),
			    Double)], dt: Double) {
    for i in 0..<bodies.count {
        for j in i+1..<bodies.count {
            let (dx, dy, dz) = (bodies[i].0.0 - bodies[j].0.0,
                                bodies[i].0.1 - bodies[j].0.1,
                                bodies[i].0.2 - bodies[j].0.2)
            
            let dSquared = dx*dx + dy*dy + dz*dz
            let distance = sqrt(dSquared)
            let mag = dt / (dSquared * distance)
            
            bodies[i].1 = (bodies[i].1.0 - dx * bodies[j].2 * mag,
                           bodies[i].1.1 - dy * bodies[j].2 * mag,
                           bodies[i].1.2 - dz * bodies[j].2 * mag)
            bodies[j].1 = (bodies[j].1.0 + dx * bodies[i].2 * mag,
                           bodies[j].1.1 + dy * bodies[i].2 * mag,
                           bodies[j].1.2 + dz * bodies[i].2 * mag)
        }
    }
    for i in 0..<bodies.count {
        bodies[i].0 = (bodies[i].0.0 + dt * bodies[i].1.0,
                       bodies[i].0.1 + dt * bodies[i].1.1,
                       bodies[i].0.2 + dt * bodies[i].1.2)
    }
}

func energy(_ bodies: [((Double,Double,Double),
		      (Double,Double,Double),
		      Double)]) -> Double {
    var energy = 0.0
    for (i, body) in bodies.enumerated() {
        energy += 0.5 * body.2 * ( body.1.0*body.1.0
				 + body.1.1*body.1.1
				 + body.1.2*body.1.2)
        for jbody in bodies[i+1..<bodies.count] {
            let dx = body.0.0 - jbody.0.0
            let dy = body.0.1 - jbody.0.1
            let dz = body.0.2 - jbody.0.2
            let distance = sqrt(dx*dx + dy*dy + dz*dz)
            energy -= (body.2 * jbody.2) / distance
        }
    }
    
    return energy
}

let n: Int
if CommandLine.argc > 1 {
    n = Int(CommandLine.arguments[1]) ?? 1000
} else {
    n = 1000
}

// Adjust momentum of the sun
var p = (0.0, 0.0, 0.0)
for body in bodies {
    p.0 += body.1.0 * body.2
    p.1 += body.1.1 * body.2
    p.2 += body.1.2 * body.2
}
bodies[0].1 = (-p.0 / SOLAR_MASS, -p.1 / SOLAR_MASS, -p.2 / SOLAR_MASS)

print("\(energy(bodies))")
for _ in 0..<n {
    advance(&bodies, dt: 0.01)
}
print("\(energy(bodies))")
