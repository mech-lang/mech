// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Cristi Cobzarenco
// vectorized by Felix Schütt

#![cfg(target_feature="sse2")]

use std::ops::{Add, Sub, Mul};
#[cfg(target_arch="x86")]
use std::arch::x86::*;
#[cfg(target_arch="x86_64")]
use std::arch::x86_64::*;

const PI: f64 = 3.141592653589793;
const SOLAR_MASS: f64 = 4.0 * PI * PI;
const DAYS_PER_YEAR: f64 = 365.24;
const N_BODIES: usize = 5;
const N_PAIRS: usize = N_BODIES * (N_BODIES - 1) / 2;

const BODIES: [Planet; N_BODIES] = [
    // Sun
    Planet {
        pos: Vec3([0.0, 0.0, 0.0, 0.0]),
        vel: Vec3([0.0, 0.0, 0.0, 0.0]),
        mass: SOLAR_MASS,
    },
    // Jupiter
    Planet {
        pos: Vec3([4.84143144246472090e+00,
                   -1.16032004402742839e+00,
                   -1.03622044471123109e-01,
                   0.0]),
        vel: Vec3([1.66007664274403694e-03 * DAYS_PER_YEAR,
                   7.69901118419740425e-03 * DAYS_PER_YEAR,
                   -6.90460016972063023e-05 * DAYS_PER_YEAR,
                   0.0]),
        mass: 9.54791938424326609e-04 * SOLAR_MASS,
    },
    // Saturn
    Planet {
        pos: Vec3([8.34336671824457987e+00,
                   4.12479856412430479e+00,
                   -4.03523417114321381e-01,
                   0.0]),
        vel: Vec3([-2.76742510726862411e-03 * DAYS_PER_YEAR,
                    4.99852801234917238e-03 * DAYS_PER_YEAR,
                    2.30417297573763929e-05 * DAYS_PER_YEAR,
                    0.0]),
        mass: 2.85885980666130812e-04 * SOLAR_MASS,
    },
    // Uranus
    Planet {
        pos: Vec3([1.28943695621391310e+01,
                   -1.51111514016986312e+01,
                   -2.23307578892655734e-01,
                   0.0]),
        vel: Vec3([2.96460137564761618e-03 * DAYS_PER_YEAR,
                   2.37847173959480950e-03 * DAYS_PER_YEAR,
                   -2.96589568540237556e-05 * DAYS_PER_YEAR,
                   0.0]),
        mass: 4.36624404335156298e-05 * SOLAR_MASS,
    },
    // Neptune
    Planet {
        pos: Vec3([1.53796971148509165e+01,
                   -2.59193146099879641e+01,
                   1.79258772950371181e-01,
                   0.0]),
        vel: Vec3([2.68067772490389322e-03 * DAYS_PER_YEAR,
                   1.62824170038242295e-03 * DAYS_PER_YEAR,
                   -9.51592254519715870e-05 * DAYS_PER_YEAR,
                   0.0]),
        mass: 5.15138902046611451e-05 * SOLAR_MASS,
    },
];

/// A 3d Vector type with oveloaded operators to improve readability.
#[repr(align(16))]
#[derive(Debug, Copy, Clone)]
struct Vec3(pub [f64;4]);

impl Vec3 {
    fn zero() -> Self { Vec3([0.0; 4]) }

    fn norm(&self) -> f64 { self.squared_norm().sqrt() }

    fn squared_norm(&self) -> f64 {

        let vec1 = unsafe { _mm_load_pd(&self.0[0]) };
        let vec2 = unsafe { _mm_load_pd(&self.0[2]) };
        
        let vec1 = unsafe { _mm_mul_pd(vec1, vec1) };
        let vec2 = unsafe { _mm_mul_pd(vec2, vec2) };
        
        let vec1 = unsafe { _mm_add_pd(vec1, vec2) };

        let lo = unsafe { _mm_cvtsd_f64(vec1) };
        let hi = unsafe { _mm_cvtsd_f64(_mm_unpackhi_pd(vec1, vec1)) };

        lo + hi
    }
}

macro_rules! simd_op {
    ($a:expr, $b:expr, $op:ident) => ({

        let vec1 = unsafe { _mm_load_pd(&$a.0[0]) };
        let vec2 = unsafe { _mm_load_pd(&$b.0[0]) };

        let combined_1 = unsafe { $op(vec1, vec2)};
        
        let vec3 = unsafe { _mm_load_pd(&$a.0[2]) };
        let vec4 = unsafe { _mm_load_pd(&$b.0[2]) };
        let combined_2 = unsafe { $op(vec3, vec4)};

        let mut target = [0.0, 0.0, 0.0, 0.0];

        unsafe { _mm_store_pd(&mut target[0], combined_1) };
        unsafe { _mm_store_pd(&mut target[2], combined_2) };

        target
    })
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Vec3(simd_op!(&self, &rhs, _mm_add_pd))
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Vec3(simd_op!(&self, &rhs, _mm_sub_pd))
    }
}

impl Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        
        let self_lo = unsafe { _mm_load_pd(&self.0[0]) };
        let self_hi = unsafe { _mm_load_pd(&self.0[2]) };
        
        let scalar = unsafe { _mm_set1_pd(rhs) };
        
        let self_lo = unsafe { _mm_mul_pd(self_lo, scalar)};
        let self_hi = unsafe { _mm_mul_pd(self_hi, scalar)};

        let mut target = [0.0, 0.0, 0.0, 0.0];

        unsafe { _mm_store_pd(&mut target[0], self_lo) };
        unsafe { _mm_store_pd(&mut target[2], self_hi) };

        Vec3(target)
    }
}

#[derive(Debug, Clone, Copy)]
struct Planet {
    pos: Vec3,
    vel: Vec3,
    mass: f64,
}

/// Computes all pairwise position differences between the planets.
fn pairwise_diffs(bodies: &[Planet; N_BODIES], diff: &mut [Vec3; N_PAIRS]) {
    let mut bodies = bodies.iter();
    let mut diff = diff.iter_mut();
    while let Some(bi) = bodies.next() {
        for bj in bodies.clone() {
            *diff.next().unwrap() = bi.pos - bj.pos;
        }
    }
}

/// Computes the magnitude of the force between each pair of planets.
fn magnitudes(diff: &[Vec3; N_PAIRS], dt: f64, mag: &mut [f64; N_PAIRS]) {
    for (mag, diff) in mag.iter_mut().zip(diff.iter()) {
        let d2 = diff.squared_norm();
        *mag = dt / (d2 * d2.sqrt());
    }
}

/// Updates the velocities of the planets by computing their gravitational
/// accelerations and performing one step of Euler integration.
fn update_velocities(bodies: &mut [Planet; N_BODIES], dt: f64,
                     diff: &mut [Vec3; N_PAIRS], mag: &mut [f64; N_PAIRS]) {
    pairwise_diffs(bodies, diff);
    magnitudes(&diff, dt, mag);

    let mut bodies = &mut bodies[..];
    let mut mag = mag.iter();
    let mut diff = diff.iter();
    while let Some(bi) = shift_mut_ref(&mut bodies) {
        for bj in bodies.iter_mut() {
            let diff = *diff.next().unwrap();
            let mag = *mag.next().unwrap();
            bi.vel = bi.vel - diff * (bj.mass * mag);
            bj.vel = bj.vel + diff * (bi.mass * mag);
        }
    }
}

/// Advances the solar system by one timestep by first updating the
/// velocities and then integrating the positions using the updated velocities.
///
/// Note: the `diff` & `mag` arrays are effectively scratch space. They're
/// provided as arguments to avoid re-zeroing them every time `advance` is
/// called.
fn advance(bodies: &mut [Planet; N_BODIES], dt: f64,
           diff: &mut [Vec3; N_PAIRS], mag: &mut [f64; N_PAIRS]) {
    update_velocities(bodies, dt, diff, mag);
    for body in bodies.iter_mut() {
        body.pos = body.pos + body.vel * dt;
    }
}

/// Computes the total energy of the solar system.
fn energy(bodies: &[Planet; N_BODIES]) -> f64 {
    let mut e = 0.0;
    let mut bodies = bodies.iter();
    while let Some(bi) = bodies.next() {
        e += bi.vel.squared_norm() * bi.mass / 2.0
           - bi.mass * bodies.clone()
                             .map(|bj| bj.mass / (bi.pos - bj.pos).norm())
                             .fold(0.0, |a, b| a + b);
    }
    e
}

/// Offsets the sun's velocity to make the overall momentum of the system zero.
fn offset_momentum(bodies: &mut [Planet; N_BODIES]) {
    let p = bodies.iter().fold(Vec3::zero(), |v, b| v + b.vel * b.mass);
    bodies[0].vel = p * (-1.0 / bodies[0].mass);
}

fn main() {
    let n = std::env::args_os().nth(1)
        .and_then(|s| s.into_string().ok())
        .and_then(|n| n.parse().ok())
        .unwrap_or(1000);
    let mut bodies = BODIES;
    let mut diff = [Vec3::zero(); N_PAIRS];
    let mut mag = [0.0f64; N_PAIRS];

    offset_momentum(&mut bodies);
    println!("{:.9}", energy(&bodies));

    for _ in 0..n {
        advance(&mut bodies, 0.01, &mut diff, &mut mag);
    }

    println!("{:.9}", energy(&bodies));
}

/// Pop a mutable reference off the head of a slice, mutating the slice to no
/// longer contain the mutable reference.
fn shift_mut_ref<'a, T>(r: &mut &'a mut [T]) -> Option<&'a mut T> {
    if r.len() == 0 { return None }
    let tmp = std::mem::replace(r, &mut []);
    let (h, t) = tmp.split_at_mut(1);
    *r = t;
    Some(&mut h[0])
}
