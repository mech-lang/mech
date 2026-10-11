/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by The Go Authors.
 * based on C program by Christoph Bauer
 * flag.Arg hack by Isaac Gouy
 * Modified by Antonio Petri
 */

package main

import (
    "flag"
    "fmt"
    "math"
    "strconv"
)

var n = 0

const (
    solarMass   = 4 * math.Pi * math.Pi
    daysPerYear = 365.24
    N           = 5
)

type Position struct{ x, y, z float64 }
type Momentum struct{ x, y, z, m float64 }
type System struct {
    v [N]Momentum
    s [N]Position
}

func offsetMomentum() {
    var px, py, pz float64
    for i := 0; i < N; i++ {
        px += sys.v[i].x * sys.v[i].m
        py += sys.v[i].y * sys.v[i].m
        pz += sys.v[i].z * sys.v[i].m
    }
    sys.v[0].x = -px / solarMass
    sys.v[0].y = -py / solarMass
    sys.v[0].z = -pz / solarMass
}

func energy() float64 {
    var e float64
    for i := 0; i < N; i++ {
        e += 0.5 * sys.v[i].m *
            (sys.v[i].x*sys.v[i].x + sys.v[i].y*sys.v[i].y + sys.v[i].z*sys.v[i].z)
        for j := i + 1; j < N; j++ {
            dx := sys.s[i].x - sys.s[j].x
            dy := sys.s[i].y - sys.s[j].y
            dz := sys.s[i].z - sys.s[j].z
            distance := math.Sqrt(dx*dx + dy*dy + dz*dz)
            e -= (sys.v[i].m * sys.v[j].m) / distance
        }
    }
    return e
}

func advance(dt float64) {

    for i := 0; i < N-1; i++ {
        _vx, _vy, _vz := sys.v[i].x, sys.v[i].y, sys.v[i].z

        for j := i + 1; j < N; j++ {

            dx := sys.s[i].x - sys.s[j].x
            dy := sys.s[i].y - sys.s[j].y
            dz := sys.s[i].z - sys.s[j].z

            dSquared := dx*dx + dy*dy + dz*dz
            distance := math.Sqrt(dSquared)
            mag := (dt / (dSquared * distance))
            mi := sys.v[i].m
            _vx -= dx * sys.v[j].m * mag
            _vy -= dy * sys.v[j].m * mag
            _vz -= dz * sys.v[j].m * mag
            sys.v[j].x += dx * mi * mag
            sys.v[j].y += dy * mi * mag
            sys.v[j].z += dz * mi * mag
        }
        sys.v[i].x, sys.v[i].y, sys.v[i].z = _vx, _vy, _vz
    }

    for i := 0; i < N; i++ {
        sys.s[i].x += dt * sys.v[i].x
        sys.s[i].y += dt * sys.v[i].y
        sys.s[i].z += dt * sys.v[i].z
    }
}

var sys = System{
    v: [N]Momentum{
        {0.0, 0.0, 0.0, solarMass},
        {
            1.66007664274403694e-03 * daysPerYear,
            7.69901118419740425e-03 * daysPerYear,
            -6.90460016972063023e-05 * daysPerYear,
            9.54791938424326609e-04 * solarMass,
        },
        {
            -2.76742510726862411e-03 * daysPerYear,
            4.99852801234917238e-03 * daysPerYear,
            2.30417297573763929e-05 * daysPerYear,
            2.85885980666130812e-04 * solarMass,
        },
        {
            2.96460137564761618e-03 * daysPerYear,
            2.37847173959480950e-03 * daysPerYear,
            -2.96589568540237556e-05 * daysPerYear,
            4.36624404335156298e-05 * solarMass,
        },
        {
            2.68067772490389322e-03 * daysPerYear,
            1.62824170038242295e-03 * daysPerYear,
            -9.51592254519715870e-05 * daysPerYear,
            5.15138902046611451e-05 * solarMass,
        },
    },

    s: [N]Position{
        {0.0, 0.0, 0.0},
        {
            4.84143144246472090e+00,
            -1.16032004402742839e+00,
            -1.03622044471123109e-01,
        },
        {
            8.34336671824457987e+00,
            4.12479856412430479e+00,
            -4.03523417114321381e-01,
        },
        {
            1.28943695621391310e+01,
            -1.51111514016986312e+01,
            -2.23307578892655734e-01,
        },
        {
            1.53796971148509165e+01,
            -2.59193146099879641e+01,
            1.79258772950371181e-01,
        },
    },
}

func main() {
    flag.Parse()
    if flag.NArg() > 0 {
        n, _ = strconv.Atoi(flag.Arg(0))
    }
    offsetMomentum()

    fmt.Printf("%.9f\n", energy())
    for i := 0; i < n; i++ {
        advance(0.01)
    }
    fmt.Printf("%.9f\n", energy())

}
