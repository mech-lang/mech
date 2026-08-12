#include <cuda.h>

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static CUcontext mech_cuda_context;

static void cuda_check(CUresult result, const char *operation) {
  if (result == CUDA_SUCCESS) {
    return;
  }
  const char *name = "unknown";
  const char *message = "unknown CUDA driver error";
  cuGetErrorName(result, &name);
  cuGetErrorString(result, &message);
  fprintf(stderr, "%s failed: %s (%s), CUDA error %d\n", operation, name,
          message, (int)result);
  abort();
}

void *mgpuModuleLoadJIT(void *data, int32_t opt_level) {
  (void)opt_level;
  CUdevice device;
  CUmodule module;
  cuda_check(cuInit(0), "cuInit");
  cuda_check(cuDeviceGet(&device, 0), "cuDeviceGet");
#if CUDA_VERSION >= 13000
  CUctxCreateParams context_params = {0};
  cuda_check(cuCtxCreate(&mech_cuda_context, &context_params, 0, device),
             "cuCtxCreate");
#else
  cuda_check(cuCtxCreate(&mech_cuda_context, 0, device), "cuCtxCreate");
#endif
  cuda_check(cuModuleLoadData(&module, data), "cuModuleLoadData");
  return module;
}

void mgpuModuleUnload(void *module) {
  CUresult unload_result = cuModuleUnload((CUmodule)module);
  if (unload_result == CUDA_ERROR_DEINITIALIZED) {
    mech_cuda_context = NULL;
    return;
  }
  cuda_check(unload_result, "cuModuleUnload");
  cuda_check(cuCtxDestroy(mech_cuda_context), "cuCtxDestroy");
  mech_cuda_context = NULL;
}

void *mgpuModuleGetFunction(void *module, void *name) {
  CUfunction function;
  cuda_check(
      cuModuleGetFunction(&function, (CUmodule)module, (const char *)name),
      "cuModuleGetFunction");
  return function;
}

void *mgpuStreamCreate(void) {
  CUstream stream;
  cuda_check(cuStreamCreate(&stream, CU_STREAM_DEFAULT), "cuStreamCreate");
  return stream;
}

void mgpuLaunchKernel(void *kernel, int64_t grid_x, int64_t grid_y,
                      int64_t grid_z, int64_t block_x, int64_t block_y,
                      int64_t block_z, int32_t shared_memory, void *stream,
                      void *params, void *extra, int64_t params_count) {
  (void)params_count;
  cuda_check(cuLaunchKernel((CUfunction)kernel, (unsigned int)grid_x,
                            (unsigned int)grid_y, (unsigned int)grid_z,
                            (unsigned int)block_x, (unsigned int)block_y,
                            (unsigned int)block_z,
                            (unsigned int)shared_memory, (CUstream)stream,
                            (void **)params, (void **)extra),
             "cuLaunchKernel");
}

void mgpuStreamSynchronize(void *stream) {
  cuda_check(cuStreamSynchronize((CUstream)stream), "cuStreamSynchronize");
}

void mgpuStreamDestroy(void *stream) {
  cuda_check(cuStreamDestroy((CUstream)stream), "cuStreamDestroy");
}
