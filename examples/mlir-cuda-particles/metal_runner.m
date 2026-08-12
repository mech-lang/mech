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

    float expected_position = 1.0f;
    float expected_velocity = 0.5f;
    for (NSUInteger ordinal = 0; ordinal < turns; ++ordinal) {
      expected_velocity += expected_position * -0.125f * 0.25f;
      expected_position += expected_velocity * 0.25f;
    }
    float *values = state.contents;
    float position_error = fabsf(values[0] - expected_position);
    float velocity_error = fabsf(values[lanes] - expected_velocity);
    double particle_turns = (double)lanes * (double)turns;

    printf("device: %s\n", device.name.UTF8String);
    printf("particles: %lu\n", (unsigned long)lanes);
    printf("turns: %lu\n", (unsigned long)turns);
    printf("resident dispatch: %.3f ms\n", elapsed * 1000.0);
    printf("resident throughput: %.3f million particle-turns/s\n",
           particle_turns / elapsed / 1.0e6);
    printf("position[0]: %.9g (error %.3e)\n", values[0], position_error);
    printf("velocity[0]: %.9g (error %.3e)\n", values[lanes],
           velocity_error);
    if (position_error > 1.0e-5f || velocity_error > 1.0e-5f) {
      fprintf(stderr, "GPU result did not match the f32 reference\n");
      return 1;
    }
  }
  return 0;
}
