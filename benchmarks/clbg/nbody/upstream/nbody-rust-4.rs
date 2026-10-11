//
// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Alex Drozhak
// modified by Tung Duong
//

use std::ops::{Add, Sub, Mul, AddAssign, SubAssign};
use std::f64::consts::PI;

const SOLAR_MASS: f64 = 4.0 * PI * PI;
const YEAR: f64 = 365.24;
const N_BODIES: usize = 5;
const N_PAIRS: usize = N_BODIES*(N_BODIES-1)/2;

#[derive(Clone, Copy)]
struct Vec3(pub f64, pub f64, pub f64);

#[derive(Clone, Copy)]
struct Pair(pub usize, pub usize);

impl Vec3 {
    fn new() -> Self {
        Vec3(0.0, 0.0, 0.0)
    }

    fn norm(&self) -> f64 {
        self.squared_norm().sqrt()
    }

    fn squared_norm(&self) -> f64 {
        self.0 * self.0 + self.1 * self.1 + self.2 * self.2
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Vec3(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2)
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Vec3(self.0 - rhs.0, self.1 - rhs.1, self.2 - rhs.2)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        *self = Vec3(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2);
    }
}

impl SubAssign for Vec3 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = Vec3(self.0 - rhs.0, self.1 - rhs.1, self.2 - rhs.2);
    }
}

impl Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Vec3(self.0 * rhs, self.1 * rhs, self.2 * rhs)
    }
}

#[derive(Clone, Copy)]
struct Planet {
    pos: Vec3,
    vel: Vec3,
    mass: f64    
}

impl Planet {
    fn sun() -> Self {
        Planet {
            pos: Vec3(0.0, 0.0, 0.0),
            vel: Vec3(0.0, 0.0, 0.0),
            mass: SOLAR_MASS,
        }
    }

    fn jupiter() -> Self {
        Planet {
            pos: Vec3(4.84143144246472090e+00,
                      -1.16032004402742839e+00,
                      -1.03622044471123109e-01),
            vel: Vec3(1.66007664274403694e-03 * YEAR,
                      7.69901118419740425e-03 * YEAR,
                      -6.90460016972063023e-05 * YEAR),
            mass: 9.54791938424326609e-04 * SOLAR_MASS,
        }
    }

    fn saturn() -> Self {
        Planet {
            pos: Vec3(8.34336671824457987e+00,
                      4.12479856412430479e+00,
                      -4.03523417114321381e-01),
            vel: Vec3(-2.76742510726862411e-03 * YEAR,
                      4.99852801234917238e-03 * YEAR,
                      2.30417297573763929e-05 * YEAR),
            mass: 2.85885980666130812e-04 * SOLAR_MASS,
        }
    }

    fn uranus() -> Self {
        Planet {
            pos: Vec3(1.28943695621391310e+01,
                      -1.51111514016986312e+01,
                      -2.23307578892655734e-01),
            vel: Vec3(2.96460137564761618e-03 * YEAR,
                      2.37847173959480950e-03 * YEAR,
                      -2.96589568540237556e-05 * YEAR),
            mass: 4.36624404335156298e-05 * SOLAR_MASS,
        }
    }

    fn neptune() -> Self {
        Planet {
            pos: Vec3(1.53796971148509165e+01,
                      -2.59193146099879641e+01,
                      1.79258772950371181e-01),
            vel: Vec3(2.68067772490389322e-03 * YEAR,
                      1.62824170038242295e-03 * YEAR,
                      -9.51592254519715870e-05 * YEAR),
            mass: 5.15138902046611451e-05 * SOLAR_MASS,
        }
    }
}

struct NBSystem {
    planets: [Planet; N_BODIES],
    pairs : [Pair; N_PAIRS],
    offsets : [Vec3; N_PAIRS],
    distances_square :[f64;N_PAIRS],
    distances :[f64;N_PAIRS],
    mags :[f64;N_PAIRS]
}

impl NBSystem {
    fn new() -> Self {
        let mut system = NBSystem {
            planets: [Planet::sun(),
                      Planet::jupiter(),
                      Planet::saturn(),
                      Planet::uranus(),
                      Planet::neptune()],
            pairs : [Pair(0,0); N_PAIRS],
            offsets : [Vec3(0.0,0.0,0.0);N_PAIRS],
            distances_square : [0.0;N_PAIRS],
            distances : [0.0;N_PAIRS],
            mags : [0.0;N_PAIRS],
        };
        let mut index = 0;
        for i in 0..N_BODIES {
            for j in (i + 1)..N_BODIES {
				system.pairs[index].0 = i;
				system.pairs[index].1 = j;
				index += 1;
			}
		}
        system
    }

    fn offset_momentum(&mut self) {
        let p = self.planets.iter_mut().fold(Vec3::new(), |v, b| v + b.vel * b.mass);
        self.planets[0].vel = p * (-1.0 / self.planets[0].mass);
    }

    fn energy(&self) -> f64 {
        let mut e = 0.0;
        let mut bodies = self.planets.iter();
        while let Some(bi) = bodies.next() {
            e +=
                bi.vel.squared_norm() * bi.mass / 2.0 -
                bi.mass *
                bodies.clone().map(|bj| bj.mass / (bi.pos - bj.pos).norm()).fold(0.0, |a, b| a + b);
        }
        e
    }

    fn advance(&mut self, dt: f64) {
        let planets = &mut self.planets;
        for (index, &Pair(i,j)) in self.pairs.iter().enumerate() {
			self.offsets[index] = planets[i].pos - planets[j].pos;
		}
		for index in 0..N_PAIRS {
			self.distances_square[index] = self.offsets[index].squared_norm();
			self.distances[index] = self.distances_square[index].sqrt();
		}
		for index in 0..N_PAIRS {
			self.mags[index] = dt / (self.distances_square[index] * self.distances[index]);
		}
        for (index, &Pair(i,j)) in self.pairs.iter().enumerate() {
		    planets[i].vel -= self.offsets[index] * planets[j].mass * self.mags[index];
		    planets[j].vel += self.offsets[index] * planets[i].mass * self.mags[index];
		}
        for planet in planets.iter_mut() {
            planet.pos += planet.vel * dt;
        }
    }
}

fn main() {
    let n = std::env::args_os()
        .nth(1)
        .and_then(|s| s.into_string().ok())
        .and_then(|n| n.parse().ok())
        .unwrap_or(1000);
    let mut system = NBSystem::new();
    system.offset_momentum();
    println!("{:.9}", system.energy());
    for _ in 0..n {
        system.advance(0.01);
    }
    println!("{:.9}", system.energy());
}
