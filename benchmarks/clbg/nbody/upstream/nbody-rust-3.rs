/// The Computer Language Benchmarks Game
/// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
/// 
/// Contributed by Ilia Schelokov

use std::f64::consts::PI;
use std::ops::{Add, Sub, Mul, AddAssign, SubAssign};
use std::default::Default;

#[derive(Clone, Debug)]
struct Vec3D(f64, f64, f64);

impl Vec3D {
    fn sum_squares(&self) -> f64 {
        self.0 * self.0
            + self.1 * self.1
            + self.2 * self.2
    }

    fn magnitude(&self, dt: f64) -> f64 {
        let sum = self.sum_squares();
        dt / (sum * sum.sqrt())
    }
}

impl Default for Vec3D {
    fn default() -> Vec3D {
        Vec3D(0.0, 0.0, 0.0)
    }
}

impl Add for &Vec3D {
    type Output = Vec3D;
    fn add(self, rhs: Self) -> Self::Output {
        Vec3D(
            self.0 + rhs.0,
            self.1 + rhs.1,
            self.2 + rhs.2
        )
    }
}

impl Sub for &Vec3D {
    type Output = Vec3D;
    fn sub(self, rhs: Self) -> Self::Output {
        Vec3D(
            self.0 - rhs.0,
            self.1 - rhs.1,
            self.2 - rhs.2
        )
    }
}

impl Mul<f64> for &Vec3D {
    type Output = Vec3D;
    fn mul(self, rhs: f64) -> Self::Output {
        Vec3D(
            self.0 * rhs,
            self.1 * rhs,
            self.2 * rhs
        )
    }
}

impl AddAssign for Vec3D {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
        self.2 += rhs.2;
    }
}

impl SubAssign for Vec3D {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
        self.2 -= rhs.2;
    }
}

#[derive(Clone, Debug)]
struct Body {
    position: Vec3D,
    velocity: Vec3D,
    mass: f64,
}

const BODIES_COUNT: usize = 5;

const SOLAR_MASS: f64 = 4. * PI * PI;
const DAYS_PER_YEAR: f64 = 365.24;

const INTERACTIONS: usize = BODIES_COUNT * (BODIES_COUNT - 1) / 2;

const STARTING_STATE: [Body; BODIES_COUNT] = [
    // Sun
    Body {
        mass: SOLAR_MASS,
        position: Vec3D(0., 0., 0.),
        velocity: Vec3D(0., 0., 0.),
    },
    // Jupiter
    Body {
        position: Vec3D(
            4.841_431_442_464_72e0,
            -1.160_320_044_027_428_4e0,
            -1.036_220_444_711_231_1e-1,
        ),
        velocity: Vec3D(
            1.660_076_642_744_037e-3 * DAYS_PER_YEAR,
            7.699_011_184_197_404e-3 * DAYS_PER_YEAR,
            -6.904_600_169_720_63e-5 * DAYS_PER_YEAR,
        ),
        mass: 9.547_919_384_243_266e-4 * SOLAR_MASS,
    },
    // Saturn
    Body {
        position: Vec3D(
            8.343_366_718_244_58e0,
            4.124_798_564_124_305e0,
            -4.035_234_171_143_214e-1,
        ),
        velocity: Vec3D(
            -2.767_425_107_268_624e-3 * DAYS_PER_YEAR,
            4.998_528_012_349_172e-3 * DAYS_PER_YEAR,
            2.304_172_975_737_639_3e-5 * DAYS_PER_YEAR,
        ),
        mass: 2.858_859_806_661_308e-4 * SOLAR_MASS,
    },
    // Uranus
    Body {
        position: Vec3D(
            1.289_436_956_213_913_1e1,
            -1.511_115_140_169_863_1e1,
            -2.233_075_788_926_557_3e-1,
        ),
        velocity: Vec3D(
            2.964_601_375_647_616e-3 * DAYS_PER_YEAR,
            2.378_471_739_594_809_5e-3 * DAYS_PER_YEAR,
            -2.965_895_685_402_375_6e-5 * DAYS_PER_YEAR,
        ),
        mass: 4.366_244_043_351_563e-5 * SOLAR_MASS,
    },
    // Neptune
    Body {
        position: Vec3D(
            1.537_969_711_485_091_1e1,
            -2.591_931_460_998_796_4e1,
            1.792_587_729_503_711_8e-1,
        ),
        velocity: Vec3D(
            2.680_677_724_903_893_2e-3 * DAYS_PER_YEAR,
            1.628_241_700_382_423e-3 * DAYS_PER_YEAR,
            -9.515_922_545_197_159e-5 * DAYS_PER_YEAR,
        ),
        mass: 5.151_389_020_466_114_5e-5 * SOLAR_MASS,
    },
];

/// Steps the simulation forward by one time-step.
fn advance(bodies: &mut [Body; BODIES_COUNT], dt: f64, steps: usize) {
    let mut d_positions: [Vec3D; INTERACTIONS] = Default::default();
    let mut magnitudes = [0.; INTERACTIONS];

    for _ in 0 .. steps {
        // Vectors between each pair of bodies.
        let mut k = 0;
        for (i, body1) in bodies.iter().enumerate() {
            for body2 in &bodies[i + 1 ..] {
                d_positions[k] = &body1.position - &body2.position;
                k += 1;
            }
        }
    
        // Magnitude between each pair of bodies.
        for (mag, d_pos) in magnitudes.iter_mut().zip(d_positions.iter()) {
            *mag = d_pos.magnitude(dt);
        };
    
        // Apply every other body's gravitation to each body's velocity.
        let mut k = 0;
        for i in 0 .. BODIES_COUNT - 1 {
            let (body1, rest) = bodies[i..].split_first_mut().unwrap();
            for body2 in rest {
                let d_pos       = &d_positions[k];
                let mag         = magnitudes[k];
                body1.velocity -= d_pos * (body2.mass * mag);
                body2.velocity += d_pos * (body1.mass * mag);
                k += 1;
            }
        }
    
        // Update positions
        for body in bodies.iter_mut() {
            body.position += &body.velocity * dt;
        }
    }
}

/// Adjust the Sun's velocity to offset system momentum.
fn offset_momentum(bodies: &mut [Body; BODIES_COUNT]) {
    let (sun, planets) = bodies.split_first_mut().unwrap();
    sun.velocity = Default::default();
    for planet in planets {
        sun.velocity -= &planet.velocity * (planet.mass / SOLAR_MASS);
    }
}

/// Print the system energy.
fn compute_energy(bodies: &mut [Body; BODIES_COUNT]) -> f64 {
    let mut energy = 0.;
    for (i, body1) in bodies.iter().enumerate() {
        // Add the kinetic energy for each body.
        energy += 0.5
            * body1.mass
            * body1.velocity.sum_squares();
        // Add the potential energy between this body and every other body.
        for body2 in &bodies[i + 1 ..] {
            let d_pos = &body1.position - &body2.position;
            energy -= body1.mass * body2.mass / d_pos.sum_squares().sqrt();
        }
    }
    energy
}

fn main() {
    let ncycles = std::env::args_os()
        .nth(1)
        .and_then(|s| s.into_string().ok())
        .and_then(|n| n.parse().ok())
        .unwrap_or(1000);

    let mut bodies = STARTING_STATE;

    offset_momentum(&mut bodies);
    println!("{:.9}", compute_energy(&mut bodies));
    advance(&mut bodies, 0.01, ncycles);
    println!("{:.9}", compute_energy(&mut bodies));
}
