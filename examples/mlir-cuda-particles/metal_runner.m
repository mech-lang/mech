#import <Foundation/Foundation.h>
#import <Metal/Metal.h>
#include <math.h>

static id<MTLComputePipelineState> load_pipeline(id<MTLDevice> device,
                                                  NSString *path,
                                                  NSString *function_name) {
  NSError *error = nil;
  NSString *source = [NSString stringWithContentsOfFile:path
                                               encoding:NSUTF8StringEncoding
                                                  error:&error];
  if (source == nil) {
    fprintf(stderr, "failed to read %s: %s\n", path.UTF8String,
            error.localizedDescription.UTF8String);
    exit(1);
  }

  id<MTLLibrary> library = [device newLibraryWithSource:source
                                                options:nil
                                                  error:&error];
  if (library == nil) {
    fprintf(stderr, "failed to compile %s: %s\n", path.UTF8String,
            error.localizedDescription.UTF8String);
    exit(1);
  }
  id<MTLFunction> function = [library newFunctionWithName:function_name];
  if (function == nil) {
    fprintf(stderr, "Metal function %s was not found in %s\n",
            function_name.UTF8String, path.UTF8String);
    exit(1);
  }
  id<MTLComputePipelineState> pipeline =
      [device newComputePipelineStateWithFunction:function error:&error];
  if (pipeline == nil) {
    fprintf(stderr, "failed to create %s pipeline: %s\n",
            function_name.UTF8String, error.localizedDescription.UTF8String);
    exit(1);
  }
  return pipeline;
}

static void require_completed(id<MTLCommandBuffer> command_buffer,
                              const char *label) {
  [command_buffer waitUntilCompleted];
  if (command_buffer.status == MTLCommandBufferStatusError) {
    fprintf(stderr, "%s failed: %s\n", label,
            command_buffer.error.localizedDescription.UTF8String);
    exit(1);
  }
}

static void dispatch(id<MTLCommandQueue> queue,
                     id<MTLComputePipelineState> pipeline,
                     id<MTLBuffer> state, NSUInteger lanes) {
  id<MTLCommandBuffer> command_buffer = [queue commandBuffer];
  id<MTLComputeCommandEncoder> encoder = [command_buffer computeCommandEncoder];
  [encoder setComputePipelineState:pipeline];
  [encoder setBuffer:state offset:0 atIndex:0];
  NSUInteger width =
      MIN((NSUInteger)256, pipeline.maxTotalThreadsPerThreadgroup);
  [encoder dispatchThreads:MTLSizeMake(lanes, 1, 1)
      threadsPerThreadgroup:MTLSizeMake(width, 1, 1)];
  [encoder endEncoding];
  [command_buffer commit];
  require_completed(command_buffer, "initialization dispatch");
}

int main(int argc, const char *argv[]) {
  @autoreleasepool {
    if (argc != 6) {
      fprintf(stderr,
              "usage: %s initialize.metal turn.metal lanes state-elements turns\n",
              argv[0]);
      return 2;
    }
    NSString *initialize_path = [NSString stringWithUTF8String:argv[1]];
    NSString *turn_path = [NSString stringWithUTF8String:argv[2]];
    NSUInteger lanes = (NSUInteger)strtoull(argv[3], NULL, 10);
    NSUInteger state_elements = (NSUInteger)strtoull(argv[4], NULL, 10);
    NSUInteger turns = (NSUInteger)strtoull(argv[5], NULL, 10);
    if (lanes == 0 || state_elements == 0 || turns == 0) {
      fprintf(stderr, "lanes, state-elements, and turns must be positive\n");
      return 2;
    }
    if (state_elements != lanes * 6) {
      fprintf(stderr, "representative particle state must have six components\n");
      return 2;
    }

    id<MTLDevice> device = MTLCreateSystemDefaultDevice();
    if (device == nil) {
      fprintf(stderr, "no Metal device is available\n");
      return 1;
    }
    id<MTLCommandQueue> queue = [device newCommandQueue];
    id<MTLComputePipelineState> initialize =
        load_pipeline(device, initialize_path, @"mech_initialize");
    id<MTLComputePipelineState> turn =
        load_pipeline(device, turn_path, @"mech_turn");
    id<MTLBuffer> state =
        [device newBufferWithLength:state_elements * sizeof(float)
                           options:MTLResourceStorageModeShared];
    if (state == nil) {
      fprintf(stderr, "failed to allocate the resident GPU state buffer\n");
      return 1;
    }

    dispatch(queue, initialize, state, lanes);
    float *values = state.contents;
    float expected[6];
    for (NSUInteger component = 0; component < 6; ++component) {
      expected[component] = values[component * lanes];
    }

    id<MTLCommandBuffer> command_buffer = [queue commandBuffer];
    id<MTLComputeCommandEncoder> encoder = [command_buffer computeCommandEncoder];
    [encoder setComputePipelineState:turn];
    [encoder setBuffer:state offset:0 atIndex:0];
    NSUInteger width =
        MIN((NSUInteger)256, turn.maxTotalThreadsPerThreadgroup);
    MTLSize grid = MTLSizeMake(lanes, 1, 1);
    MTLSize group = MTLSizeMake(width, 1, 1);
    for (NSUInteger ordinal = 0; ordinal < turns; ++ordinal) {
      [encoder dispatchThreads:grid threadsPerThreadgroup:group];
      if (ordinal + 1 < turns) {
        [encoder memoryBarrierWithScope:MTLBarrierScopeBuffers];
      }
    }
    [encoder endEncoding];

    NSTimeInterval started = [NSDate timeIntervalSinceReferenceDate];
    [command_buffer commit];
    require_completed(command_buffer, "resident turn dispatches");
    NSTimeInterval elapsed = [NSDate timeIntervalSinceReferenceDate] - started;

    for (NSUInteger ordinal = 0; ordinal < turns; ++ordinal) {
      float x = expected[0];
      float y = expected[1];
      float z = expected[2];
      float vx = expected[3];
      float vy = expected[4];
      float vz = expected[5];
      float r2 = x * x + y * y + z * z + 0.75f;
      float inverse = 1.0f / r2;
      float inverse2 = inverse * inverse;
      float radial = -0.2f * inverse2;
      float ax = x * radial + y * 0.03f;
      float ay = y * radial - x * 0.03f;
      float az = z * radial;
      float next_vx = (vx + ax * 0.004f) * 0.9995f;
      float next_vy = (vy + ay * 0.004f) * 0.9995f;
      float next_vz = (vz + az * 0.004f) * 0.9995f;
      expected[0] = x + next_vx * 0.004f;
      expected[1] = y + next_vy * 0.004f;
      expected[2] = z + next_vz * 0.004f;
      expected[3] = next_vx;
      expected[4] = next_vy;
      expected[5] = next_vz;
    }
    float maximum_error = 0.0f;
    for (NSUInteger component = 0; component < 6; ++component) {
      for (NSUInteger lane = 0; lane < lanes; ++lane) {
        maximum_error = fmaxf(
            maximum_error,
            fabsf(values[component * lanes + lane] - expected[component]));
      }
    }
    double particle_turns = (double)lanes * (double)turns;

    printf("device: %s\n", device.name.UTF8String);
    printf("particles: %lu\n", (unsigned long)lanes);
    printf("turns: %lu\n", (unsigned long)turns);
    printf("resident dispatch: %.3f ms\n", elapsed * 1000.0);
    printf("resident throughput: %.3f million particle-turns/s\n",
           particle_turns / elapsed / 1.0e6);
    printf("maximum f32 absolute error: %.3e\n", maximum_error);
    printf("benchmark_csv,gpu,f32,%lu,%.9f,%.3f\n",
           (unsigned long)turns, elapsed,
           particle_turns / elapsed / 1.0e6);
    if (maximum_error > 1.0e-5f) {
      fprintf(stderr, "GPU result did not match the f32 reference\n");
      return 1;
    }
  }
  return 0;
}
