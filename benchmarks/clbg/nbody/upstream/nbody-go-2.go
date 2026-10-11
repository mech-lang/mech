/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by The Go Authors.
 * based on C program by Christoph Bauer
 * flag.Arg hack by Isaac Gouy
 * modified by Bert Gijsbers.
 */

package main

import (
    "flag"
    "fmt"
    "math"
    "strconv"
)

type Body struct {
    x, y, z, pad, vx, vy, vz, mass float64
}

const (
    solarMass   = 4 * math.Pi * math.Pi
    daysPerYear = 365.24
    numBodies   = 5
)

var system = [numBodies]Body{sun, jupiter, saturn, uranus, neptune}

func momentum() {
    var px, py, pz float64
    for i := range system {
        px += system[i].vx * system[i].mass
        py += system[i].vy * system[i].mass
        pz += system[i].vz * system[i].mass
    }
    system[0].vx = -px / solarMass
    system[0].vy = -py / solarMass
    system[0].vz = -pz / solarMass
}

func energy() float64 {
    var e float64
    for i := range system {
        e += 0.5 * system[i].mass *
            (system[i].vx*system[i].vx +
                system[i].vy*system[i].vy +
                system[i].vz*system[i].vz)
        for j := i + 1; j < numBodies; j++ {
            dx := system[i].x - system[j].x
            dy := system[i].y - system[j].y
            dz := system[i].z - system[j].z
            distance := math.Sqrt(dx*dx + dy*dy + dz*dz)
            e -= system[i].mass * system[j].mass / distance
        }
    }
    return e
}

func advance(dt float64) {
    for i := range system {
        for j := i + 1; j < numBodies; j++ {
            dx := system[i].x - system[j].x
            dy := system[i].y - system[j].y
            dz := system[i].z - system[j].z

            dSquared := dx*dx + dy*dy + dz*dz
            distance := math.Sqrt(dSquared)
            mag := dt / (dSquared * distance)

            system[i].vx -= dx * system[j].mass * mag
            system[i].vy -= dy * system[j].mass * mag
            system[i].vz -= dz * system[j].mass * mag

            system[j].vx += dx * system[i].mass * mag
            system[j].vy += dy * system[i].mass * mag
            system[j].vz += dz * system[i].mass * mag
        }
    }

    for i := range system {
        system[i].x += dt * system[i].vx
        system[i].y += dt * system[i].vy
        system[i].z += dt * system[i].vz
    }
}

var (
    jupiter = Body{
        x:    4.84143144246472090e+00,
        y:    -1.16032004402742839e+00,
        z:    -1.03622044471123109e-01,
        vx:   1.66007664274403694e-03 * daysPerYear,
        vy:   7.69901118419740425e-03 * daysPerYear,
        vz:   -6.90460016972063023e-05 * daysPerYear,
        mass: 9.54791938424326609e-04 * solarMass,
    }
    saturn = Body{
        x:    8.34336671824457987e+00,
        y:    4.12479856412430479e+00,
        z:    -4.03523417114321381e-01,
        vx:   -2.76742510726862411e-03 * daysPerYear,
        vy:   4.99852801234917238e-03 * daysPerYear,
        vz:   2.30417297573763929e-05 * daysPerYear,
        mass: 2.85885980666130812e-04 * solarMass,
    }
    uranus = Body{
        x:    1.28943695621391310e+01,
        y:    -1.51111514016986312e+01,
        z:    -2.23307578892655734e-01,
        vx:   2.96460137564761618e-03 * daysPerYear,
        vy:   2.37847173959480950e-03 * daysPerYear,
        vz:   -2.96589568540237556e-05 * daysPerYear,
        mass: 4.36624404335156298e-05 * solarMass,
    }
    neptune = Body{
        x:    1.53796971148509165e+01,
        y:    -2.59193146099879641e+01,
        z:    1.79258772950371181e-01,
        vx:   2.68067772490389322e-03 * daysPerYear,
        vy:   1.62824170038242295e-03 * daysPerYear,
        vz:   -9.51592254519715870e-05 * daysPerYear,
        mass: 5.15138902046611451e-05 * solarMass,
    }
    sun = Body{
        mass: solarMass,
    }
)

func main() {
    n := 0
    flag.Parse()
    if flag.NArg() > 0 {
        n, _ = strconv.Atoi(flag.Arg(0))
    }

    momentum()
    fmt.Printf("%.9f\n", energy())
    for i := 0; i < n; i++ {
        advance(0.01)
    }
    fmt.Printf("%.9f\n", energy())
}
