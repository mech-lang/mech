#define _GNU_SOURCE
#include <math.h>

void mech_aot_sinf_f32x4(const float *input, float *output) {
  for (unsigned lane = 0; lane < 4; ++lane) {
    output[lane] = sinf(input[lane]);
  }
}

void mech_aot_cosf_f32x4(const float *input, float *output) {
  for (unsigned lane = 0; lane < 4; ++lane) {
    output[lane] = cosf(input[lane]);
  }
}

void mech_aot_sincos_f32x4(const float *input, float *sin_output,
                           float *cos_output) {
  for (unsigned lane = 0; lane < 4; ++lane) {
#if defined(__APPLE__)
    __sincosf(input[lane], &sin_output[lane], &cos_output[lane]);
#elif defined(__GLIBC__)
    sincosf(input[lane], &sin_output[lane], &cos_output[lane]);
#else
    sin_output[lane] = sinf(input[lane]);
    cos_output[lane] = cosf(input[lane]);
#endif
  }
}

void mech_aot_atan2_f32x4(const float *left, const float *right,
                          float *output) {
  for (unsigned lane = 0; lane < 4; ++lane) {
    output[lane] = atan2f(left[lane], right[lane]);
  }
}
