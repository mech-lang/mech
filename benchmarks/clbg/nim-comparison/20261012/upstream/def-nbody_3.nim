# By tstm

import math, strutils, os

type
  Body = object
    x, y, z, vx, vy, vz, mass: float

let
  SOLAR_MASS = 4 * PI * PI
  DAYS_PER_YEAR = 365.24

  sun = Body(x: 0, y: 0, z: 0, vx: 0, vy: 0, vz: 0, mass: SOLAR_MASS)

  jupiter = Body(
    x: 4.84143144246472090,
    y: -1.16032004402742839,
    z: -1.03622044471123109e-01,
    vx: 1.66007664274403694e-03 * DAYS_PER_YEAR,
    vy: 7.69901118419740425e-03 * DAYS_PER_YEAR,
    vz: -6.90460016972063023e-05 * DAYS_PER_YEAR,
    mass: 9.54791938424326609e-04 * SOLAR_MASS)

  saturn = Body(
    x: 8.34336671824457987e+00,
    y: 4.12479856412430479e+00,
    z: -4.03523417114321381e-01,
    vx: -2.76742510726862411e-03 * DAYS_PER_YEAR,
    vy: 4.99852801234917238e-03 * DAYS_PER_YEAR,
    vz: 2.30417297573763929e-05 * DAYS_PER_YEAR,
    mass: 2.85885980666130812e-04 * SOLAR_MASS)

  uranus = Body(
    x:  1.28943695621391310e+01,
    y: -1.51111514016986312e+01,
    z: -2.23307578892655734e-01,
    vx: 2.96460137564761618e-03  * DAYS_PER_YEAR,
    vy: 2.37847173959480950e-03  * DAYS_PER_YEAR,
    vz: -2.96589568540237556e-05 * DAYS_PER_YEAR,
    mass: 4.36624404335156298e-05 * SOLAR_MASS)

  neptune = Body(
    x: 1.53796971148509165e+01,
    y: -2.59193146099879641e+01,
    z: 1.79258772950371181e-01,
    vx: 2.68067772490389322e-03  * DAYS_PER_YEAR,
    vy: 1.62824170038242295e-03  * DAYS_PER_YEAR,
    vz: -9.51592254519715870e-05 * DAYS_PER_YEAR,
    mass: 5.15138902046611451e-05 * SOLAR_MASS)

var bodies = [sun, jupiter, saturn, uranus, neptune]

proc offsetMomentum(b: var Body, px, py, pz: float) =
  b.vx = -px / SOLAR_MASS
  b.vy = -py / SOLAR_MASS
  b.vz = -pz / SOLAR_MASS

proc advance(dt: float) =
  for i in 0 .. bodies.high:
    for j in i+1 .. bodies.high:
      let
        dx = bodies[i].x - bodies[j].x
        dy = bodies[i].y - bodies[j].y
        dz = bodies[i].z - bodies[j].z
        dSquared = dx * dx + dy * dy + dz * dz
        distance = sqrt(dSquared)
        mag = dt / (dSquared * distance)

      bodies[i].vx -= dx * bodies[j].mass * mag
      bodies[i].vy -= dy * bodies[j].mass * mag
      bodies[i].vz -= dz * bodies[j].mass * mag

      bodies[j].vx += dx * bodies[i].mass * mag
      bodies[j].vy += dy * bodies[i].mass * mag
      bodies[j].vz += dz * bodies[i].mass * mag

  for body in bodies.mitems:
    body.x += dt * body.vx
    body.y += dt * body.vy
    body.z += dt * body.vz

proc energy: float =
  var dx, dy, dz, distance: float

  for i in 0 .. bodies.high:
    result += 0.5 * bodies[i].mass *
      (bodies[i].vx * bodies[i].vx +
       bodies[i].vy * bodies[i].vy +
       bodies[i].vz * bodies[i].vz)

    for j in i+1 .. bodies.high:
      dx = bodies[i].x - bodies[j].x
      dy = bodies[i].y - bodies[j].y
      dz = bodies[i].z - bodies[j].z

      distance = sqrt(dx * dx + dy * dy + dz * dz)
      result -= (bodies[i].mass * bodies[j].mass) / distance

var px, py, pz = 0.0

for body in bodies:
  px += body.vx * body.mass
  py += body.vy * body.mass
  pz += body.vz * body.mass

bodies[0].offsetMomentum(px, py, pz)

var n = paramStr(1).parseInt

echo energy().formatFloat(precision = 9)

for i in 1..n:
  advance(0.01)

echo energy().formatFloat(precision = 9)
