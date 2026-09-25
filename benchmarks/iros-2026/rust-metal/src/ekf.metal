#include <metal_stdlib>
using namespace metal;

constant uint COMPONENTS = 12;
constant float DT = 0.1f;
constant float R = 0.25f;
constant float SYMMETRY_TOLERANCE = 0.0001f;

struct Candidate {
  float x0, x1, x2;
  float p00, p01, p02, p10, p11, p12, p20, p21, p22;
};

inline Candidate ekf_candidate(device const float *state,
                               device const float *velocity,
                               device const float *angular_velocity,
                               device const float *bearing, uint n, uint i) {
  const float sx0 = state[0 * n + i];
  const float sx1 = state[1 * n + i];
  const float sx2 = state[2 * n + i];
  const float sp00 = state[3 * n + i];
  const float sp01 = state[4 * n + i];
  const float sp02 = state[5 * n + i];
  const float sp10 = state[6 * n + i];
  const float sp11 = state[7 * n + i];
  const float sp12 = state[8 * n + i];
  const float sp20 = state[9 * n + i];
  const float sp21 = state[10 * n + i];
  const float sp22 = state[11 * n + i];

  float ct;
  const float st = sincos(sx2, ct);
  const float d = velocity[i] * DT;
  const float nx0 = fma(d, ct, sx0);
  const float nx1 = fma(d, st, sx1);
  const float nx2 = fma(angular_velocity[i], DT, sx2);
  const float f02 = -d * st;
  const float f12 = d * ct;

  const float ap0 = fma(f02, sp20, sp00);
  const float ap1 = fma(f02, sp21, sp01);
  const float ap2 = fma(f02, sp22, sp02);
  const float aq0 = fma(f12, sp20, sp10);
  const float aq1 = fma(f12, sp21, sp11);
  const float aq2 = fma(f12, sp22, sp12);
  const float q00 = ct * ct * 0.0001f;
  const float q01 = ct * st * 0.0001f;
  const float q11 = st * st * 0.0001f;
  const float a00 = fma(ap2, f02, ap0) + q00;
  const float a01 = fma(ap2, f12, ap1) + q01;
  const float a02 = ap2;
  const float a10 = fma(aq2, f02, aq0) + q01;
  const float a11 = fma(aq2, f12, aq1) + q11;
  const float a12 = aq2;
  const float a20 = fma(sp22, f02, sp20);
  const float a21 = fma(sp22, f12, sp21);
  const float a22 = sp22 + 0.000025f;

  const float dx = 140.0f - nx0;
  const float dy = 12.0f - nx1;
  const float rr = fma(dx, dx, dy * dy);
  const float raw = bearing[i] - (atan2(dy, dx) - nx2);
  float raw_cos;
  const float raw_sin = sincos(raw, raw_cos);
  const float innovation = atan2(raw_sin, raw_cos);
  const float h0 = dy / rr;
  const float h1 = -dx / rr;
  const float h2 = -1.0f;
  const float ph0 = fma(a00, h0, fma(a01, h1, a02 * h2));
  const float ph1 = fma(a10, h0, fma(a11, h1, a12 * h2));
  const float ph2 = fma(a20, h0, fma(a21, h1, a22 * h2));
  const float variance = fma(h0, ph0, fma(h1, ph1, fma(h2, ph2, R)));
  const float k0 = ph0 / variance;
  const float k1 = ph1 / variance;
  const float k2 = ph2 / variance;
  const float b00 = 1.0f - k0 * h0;
  const float b01 = -k0 * h1;
  const float b02 = -k0 * h2;
  const float b10 = -k1 * h0;
  const float b11 = 1.0f - k1 * h1;
  const float b12 = -k1 * h2;
  const float b20 = -k2 * h0;
  const float b21 = -k2 * h1;
  const float b22 = 1.0f - k2 * h2;
  const float c00 = fma(b00, a00, fma(b01, a10, b02 * a20));
  const float c01 = fma(b00, a01, fma(b01, a11, b02 * a21));
  const float c02 = fma(b00, a02, fma(b01, a12, b02 * a22));
  const float c10 = fma(b10, a00, fma(b11, a10, b12 * a20));
  const float c11 = fma(b10, a01, fma(b11, a11, b12 * a21));
  const float c12 = fma(b10, a02, fma(b11, a12, b12 * a22));
  const float c20 = fma(b20, a00, fma(b21, a10, b22 * a20));
  const float c21 = fma(b20, a01, fma(b21, a11, b22 * a21));
  const float c22 = fma(b20, a02, fma(b21, a12, b22 * a22));

  Candidate out;
  out.x0 = fma(k0, innovation, nx0);
  out.x1 = fma(k1, innovation, nx1);
  out.x2 = fma(k2, innovation, nx2);
  out.p00 = fma(c00, b00, fma(c01, b01, fma(c02, b02, k0 * k0 * R)));
  out.p01 = fma(c00, b10, fma(c01, b11, fma(c02, b12, k0 * k1 * R)));
  out.p02 = fma(c00, b20, fma(c01, b21, fma(c02, b22, k0 * k2 * R)));
  out.p10 = fma(c10, b00, fma(c11, b01, fma(c12, b02, k1 * k0 * R)));
  out.p11 = fma(c10, b10, fma(c11, b11, fma(c12, b12, k1 * k1 * R)));
  out.p12 = fma(c10, b20, fma(c11, b21, fma(c12, b22, k1 * k2 * R)));
  out.p20 = fma(c20, b00, fma(c21, b01, fma(c22, b02, k2 * k0 * R)));
  out.p21 = fma(c20, b10, fma(c21, b11, fma(c22, b12, k2 * k1 * R)));
  out.p22 = fma(c20, b20, fma(c21, b21, fma(c22, b22, k2 * k2 * R)));
  return out;
}

inline void store_candidate(device float *state, uint n, uint i,
                            Candidate value) {
  state[0 * n + i] = value.x0;
  state[1 * n + i] = value.x1;
  state[2 * n + i] = value.x2;
  state[3 * n + i] = value.p00;
  state[4 * n + i] = value.p01;
  state[5 * n + i] = value.p02;
  state[6 * n + i] = value.p10;
  state[7 * n + i] = value.p11;
  state[8 * n + i] = value.p12;
  state[9 * n + i] = value.p20;
  state[10 * n + i] = value.p21;
  state[11 * n + i] = value.p22;
}

kernel void ekf_unchecked(device const float *state [[buffer(0)]],
                          device float *next_state [[buffer(1)]],
                          device const float *velocity [[buffer(2)]],
                          device const float *angular_velocity [[buffer(3)]],
                          device const float *bearing [[buffer(4)]],
                          constant uint &n [[buffer(5)]],
                          uint i [[thread_position_in_grid]]) {
  if (i < n) {
    store_candidate(next_state, n, i,
                    ekf_candidate(state, velocity, angular_velocity, bearing, n, i));
  }
}

kernel void ekf_checked(device const float *state [[buffer(0)]],
                        device float *next_state [[buffer(1)]],
                        device const float *velocity [[buffer(2)]],
                        device const float *angular_velocity [[buffer(3)]],
                        device const float *bearing [[buffer(4)]],
                        device atomic_uint *fault [[buffer(5)]],
                        constant uint &n [[buffer(6)]],
                        uint i [[thread_position_in_grid]]) {
  if (i >= n) return;
  Candidate value = ekf_candidate(state, velocity, angular_velocity, bearing, n, i);
  const bool finite = isfinite(value.x0) && isfinite(value.x1) && isfinite(value.x2) &&
      isfinite(value.p00) && isfinite(value.p01) && isfinite(value.p02) &&
      isfinite(value.p10) && isfinite(value.p11) && isfinite(value.p12) &&
      isfinite(value.p20) && isfinite(value.p21) && isfinite(value.p22);
  const bool positive = value.p00 > 0.0f && value.p11 > 0.0f && value.p22 > 0.0f;
  const bool symmetric = fabs(value.p01 - value.p10) <= SYMMETRY_TOLERANCE &&
      fabs(value.p02 - value.p20) <= SYMMETRY_TOLERANCE &&
      fabs(value.p12 - value.p21) <= SYMMETRY_TOLERANCE;
  const uint code = !finite ? 1u : (!positive ? 2u : (!symmetric ? 3u : 0u));
  if (code == 0u) {
    store_candidate(next_state, n, i, value);
  } else {
    atomic_fetch_add_explicit(&fault[0], 1u, memory_order_relaxed);
    atomic_fetch_min_explicit(&fault[1], (i << 8) | code, memory_order_relaxed);
  }
}
