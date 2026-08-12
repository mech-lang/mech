#include <cuda.h>

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct {
  double *allocated;
  double *aligned;
  intptr_t offset;
  intptr_t sizes[1];
  intptr_t strides[1];
} MemRef1D;

extern int64_t mech_state_len(void);
extern int64_t mech_batch_len(void);
extern void _mlir_ciface_mech_initialize(MemRef1D *state);
extern void _mlir_ciface_mech_launch(MemRef1D *state);

static void cuda_check(CUresult result, const char *operation) {
  if (result == CUDA_SUCCESS) {
    return;
  }
  const char *name = "unknown";
  const char *message = "unknown CUDA driver error";
  cuGetErrorName(result, &name);
  cuGetErrorString(result, &message);
  fprintf(stderr, "%s failed: %s (%s)\n", operation, name, message);
  exit(1);
}

int main(void) {
  const int64_t state_len = mech_state_len();
  const int64_t lanes = mech_batch_len();
  if (state_len != lanes * 6) {
    fprintf(stderr, "representative particle state must have six components\n");
    return 1;
  }
  const size_t state_bytes = (size_t)state_len * sizeof(double);
  double *host_state = calloc((size_t)state_len, sizeof(double));
  double *initial_state = calloc((size_t)state_len, sizeof(double));
  if (host_state == NULL || initial_state == NULL) {
    fprintf(stderr, "host state allocation failed\n");
    return 1;
  }

  MemRef1D host = {host_state, host_state, 0, {state_len}, {1}};
  _mlir_ciface_mech_initialize(&host);
  for (int64_t i = 0; i < state_len; ++i) {
    initial_state[i] = host_state[i];
  }

  CUdeviceptr device_state;
  cuda_check(cuMemAlloc(&device_state, state_bytes), "cuMemAlloc");
  cuda_check(cuMemcpyHtoD(device_state, host_state, state_bytes),
             "cuMemcpyHtoD");

  double *device_pointer = (double *)(uintptr_t)device_state;
  MemRef1D device = {device_pointer, device_pointer, 0, {state_len}, {1}};
  _mlir_ciface_mech_launch(&device);

  cuda_check(cuMemcpyDtoH(host_state, device_state, state_bytes),
             "cuMemcpyDtoH");
  cuda_check(cuMemFree(device_state), "cuMemFree");

  double maximum_error = 0.0;
  for (int64_t lane = 0; lane < lanes; ++lane) {
    const double x = initial_state[lane];
    const double y = initial_state[lanes + lane];
    const double z = initial_state[2 * lanes + lane];
    const double vx = initial_state[3 * lanes + lane];
    const double vy = initial_state[4 * lanes + lane];
    const double vz = initial_state[5 * lanes + lane];
    const double r2 = x * x + y * y + z * z + 0.75;
    const double inverse = 1.0 / r2;
    const double inverse2 = inverse * inverse;
    const double radial = -0.2 * inverse2;
    const double ax = x * radial + y * 0.03;
    const double ay = y * radial - x * 0.03;
    const double az = z * radial;
    const double next_vx = (vx + ax * 0.004) * 0.9995;
    const double next_vy = (vy + ay * 0.004) * 0.9995;
    const double next_vz = (vz + az * 0.004) * 0.9995;
    const double expected[6] = {
        x + next_vx * 0.004,
        y + next_vy * 0.004,
        z + next_vz * 0.004,
        next_vx,
        next_vy,
        next_vz,
    };
    for (int64_t component = 0; component < 6; ++component) {
      const double error =
          fabs(host_state[component * lanes + lane] - expected[component]);
      if (error > maximum_error) {
        maximum_error = error;
      }
    }
  }

  char device_name[256];
  CUdevice cuda_device;
  cuda_check(cuDeviceGet(&cuda_device, 0), "cuDeviceGet");
  cuda_check(cuDeviceGetName(device_name, sizeof(device_name), cuda_device),
             "cuDeviceGetName");
  printf("adapter: %s\n", device_name);
  printf("particle lanes: %lld\n", (long long)lanes);
  printf("resident f64 values: %lld\n", (long long)state_len);
  printf("maximum absolute error: %.3e\n", maximum_error);

  free(initial_state);
  free(host_state);
  return maximum_error <= 1e-12 ? 0 : 1;
}
