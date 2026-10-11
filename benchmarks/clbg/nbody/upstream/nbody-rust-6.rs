// The Computer Language Benchmarks Game
// http://benchmarksgame.alioth.debian.org/
//
// contributed by the packed_simd developers
// contributed by Andre Bogus


use std::f64::consts::PI;
use std::ops::*;

const SOLAR_MASS: f64 = 4.0 * PI * PI;
const DAYS_PER_YEAR: f64 = 365.24;

#[derive(Copy, Clone)]
pub struct F64x2(f64, f64);

#[derive(Copy, Clone)]
pub struct F64x4(f64, f64, f64, f64);

impl F64x2 {
    pub fn splat(x: f64) -> F64x2 { F64x2(x, x) }
    pub fn new(a: f64, b: f64) -> F64x2 { F64x2(a, b) }
    pub fn sqrt(self) -> F64x2 { F64x2(self.0.sqrt(), self.1.sqrt()) }
    pub fn write_to_slice_unaligned(self, slice: &mut [f64]) {
        slice[0] = self.0;
        slice[1] = self.1;
    }
}

impl F64x4 {
    pub fn splat(x: f64) -> F64x4 { F64x4(x, x, x, x) }
    pub fn new(a: f64, b: f64, c: f64, d: f64) -> F64x4 { F64x4(a, b, c, d) }
    pub fn sum(self) -> f64 {
        (self.0 + self.1) + (self.2 + self.3)
    }
}

impl Mul for F64x2 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        F64x2(self.0 * rhs.0, self.1 * rhs.1)
    }
}
impl Div for F64x2 {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        F64x2(self.0 / rhs.0, self.1 / rhs.1)
    }
}

impl Add for F64x4 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        F64x4(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2, self.3 + rhs.3)
    }
}
impl Sub for F64x4 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        F64x4(self.0 - rhs.0, self.1 - rhs.1, self.2 - rhs.2, self.3 - rhs.3)
    }
}
impl Mul for F64x4 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        F64x4(self.0 * rhs.0, self.1 * rhs.1, self.2 * rhs.2, self.3 * rhs.3)
    }
}

impl Mul<f64> for F64x4 {
    type Output = F64x4;
    fn mul(self, rhs: f64) -> F64x4 { self * F64x4::splat(rhs) }
}

pub struct Body {
    pub x: F64x4,
    pub v: F64x4,
    pub mass: f64,
}

const N_BODIES: usize = 5;
fn bodies() -> [Body; N_BODIES] {
    [
        // sun:
        Body {
            x: F64x4::new(0., 0., 0., 0.),
            v: F64x4::new(0., 0., 0., 0.),
            mass: SOLAR_MASS,
        },
        // jupiter:
        Body {
            x: F64x4::new(
                4.84143144246472090e+00,
                -1.16032004402742839e+00,
                -1.03622044471123109e-01,
                0.,
            ),
            v: F64x4::new(
                1.66007664274403694e-03 * DAYS_PER_YEAR,
                7.69901118419740425e-03 * DAYS_PER_YEAR,
                -6.90460016972063023e-05 * DAYS_PER_YEAR,
                0.,
            ),
            mass: 9.54791938424326609e-04 * SOLAR_MASS,
        },
        // saturn:
        Body {
            x: F64x4::new(
                8.34336671824457987e+00,
                4.12479856412430479e+00,
                -4.03523417114321381e-01,
                0.,
            ),
            v: F64x4::new(
                -2.76742510726862411e-03 * DAYS_PER_YEAR,
                4.99852801234917238e-03 * DAYS_PER_YEAR,
                2.30417297573763929e-05 * DAYS_PER_YEAR,
                0.,
            ),
            mass: 2.85885980666130812e-04 * SOLAR_MASS,
        },
        // uranus:
        Body {
            x: F64x4::new(
                1.28943695621391310e+01,
                -1.51111514016986312e+01,
                -2.23307578892655734e-01,
                0.,
            ),
            v: F64x4::new(
                2.96460137564761618e-03 * DAYS_PER_YEAR,
                2.37847173959480950e-03 * DAYS_PER_YEAR,
                -2.96589568540237556e-05 * DAYS_PER_YEAR,
                0.,
            ),
            mass: 4.36624404335156298e-05 * SOLAR_MASS,
        },
        // neptune:
        Body {
            x: F64x4::new(
                1.53796971148509165e+01,
                -2.59193146099879641e+01,
                1.79258772950371181e-01,
                0.,
            ),
            v: F64x4::new(
                2.68067772490389322e-03 * DAYS_PER_YEAR,
                1.62824170038242295e-03 * DAYS_PER_YEAR,
                -9.51592254519715870e-05 * DAYS_PER_YEAR,
                0.,
            ),
            mass: 5.15138902046611451e-05 * SOLAR_MASS,
        },
    ]
}

pub fn offset_momentum(bodies: &mut [Body; N_BODIES]) {
    let (sun, rest) = bodies.split_at_mut(1);
    let sun = &mut sun[0];
    for body in rest {
        let m_ratio = body.mass / SOLAR_MASS;
        sun.v = sun.v - body.v * m_ratio;
    }
}

pub fn energy(bodies: &[Body; N_BODIES]) -> f64 {
    let mut e = 0.;
    for i in 0..N_BODIES {
        let bi = &bodies[i];
        e += bi.mass * (bi.v * bi.v).sum() * 0.5;
        for bj in &bodies[i + 1..] {
            let dx = bi.x - bj.x;
            e -= bi.mass * bj.mass / (dx * dx).sum().sqrt()
        }
    }
    e
}

pub fn advance(bodies: &mut [Body; N_BODIES], dt: f64) {
    const N: usize = N_BODIES * (N_BODIES - 1) / 2;

    // compute distance between bodies:
    let mut r = [F64x4::splat(0.); N];
    {
        let mut i = 0;
        for j in 0..N_BODIES {
            for k in j + 1..N_BODIES {
                r[i] = bodies[j].x - bodies[k].x;
                i += 1;
            }
        }
    }

    let mut mag = [0.0; N];
    let mut i = 0;
    while i < N {
        let d2s = F64x2::new((r[i] * r[i]).sum(), (r[i + 1] * r[i + 1]).sum());
        let dmags = F64x2::splat(dt) / (d2s * d2s.sqrt());
        dmags.write_to_slice_unaligned(&mut mag[i..]);
        i += 2;
    }

    i = 0;
    for j in 0..N_BODIES {
        for k in j + 1..N_BODIES {
            let f = r[i] * mag[i];
            bodies[j].v = bodies[j].v - f * bodies[k].mass;
            bodies[k].v = bodies[k].v + f * bodies[j].mass;
            i += 1
        }
    }
    for body in bodies {
        body.x = body.x + body.v * dt;
    }
}

fn run(n: usize) -> (f64, f64) {
    let mut bodies = bodies();
    offset_momentum(&mut bodies);
    let energy_before = energy(&bodies);
    for _ in 0..n {
        advance(&mut bodies, 0.01);
    }
    let energy_after = energy(&bodies);
    (energy_before, energy_after)
}

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(1000);
    let (energy_before, energy_after) = run(n);
    println!("{:.9}\n{:.9}", energy_before, energy_after);
}
