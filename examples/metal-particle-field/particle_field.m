#import <Cocoa/Cocoa.h>
#import <Metal/Metal.h>
#import <MetalKit/MetalKit.h>
#include <math.h>

typedef struct {
  uint32_t x_offset;
  uint32_t y_offset;
  float camera_x;
  float camera_y;
  float zoom;
  float aspect;
  float point_size;
  float padding;
} RenderConfig;

@class ParticleRenderer;

@interface ParticleView : MTKView
@property(nonatomic, weak) ParticleRenderer *input_controller;
@end

@interface ParticleRenderer : NSObject <MTKViewDelegate>
- (instancetype)initWithView:(ParticleView *)view
                     device:(id<MTLDevice>)device
                computePath:(NSString *)compute_path
                 renderPath:(NSString *)render_path
              initialState:(NSString *)initial_state_path
                       lanes:(NSUInteger)lanes
              stateElements:(NSUInteger)state_elements
                laneOffsets:(NSArray<NSNumber *> *)lane_offsets
              scalarOffsets:(NSArray<NSNumber *> *)scalar_offsets;
- (void)pointerMoved:(NSEvent *)event;
- (void)pointerDown:(NSEvent *)event;
- (void)pointerUp:(NSEvent *)event;
- (void)pan:(NSEvent *)event;
- (void)zoom:(NSEvent *)event;
@end

@implementation ParticleView {
  NSTrackingArea *_tracking_area;
}

- (BOOL)acceptsFirstResponder {
  return YES;
}

- (void)updateTrackingAreas {
  if (_tracking_area != nil) {
    [self removeTrackingArea:_tracking_area];
  }
  _tracking_area = [[NSTrackingArea alloc]
      initWithRect:NSZeroRect
           options:NSTrackingMouseMoved | NSTrackingActiveInKeyWindow |
                   NSTrackingInVisibleRect
             owner:self
          userInfo:nil];
  [self addTrackingArea:_tracking_area];
  [super updateTrackingAreas];
}

- (void)mouseMoved:(NSEvent *)event {
  [self.input_controller pointerMoved:event];
}

- (void)mouseDragged:(NSEvent *)event {
  [self.input_controller pointerMoved:event];
}

- (void)mouseDown:(NSEvent *)event {
  [self.input_controller pointerDown:event];
}

- (void)mouseUp:(NSEvent *)event {
  [self.input_controller pointerUp:event];
}

- (void)rightMouseDragged:(NSEvent *)event {
  [self.input_controller pan:event];
}

- (void)scrollWheel:(NSEvent *)event {
  [self.input_controller zoom:event];
}
@end

static NSString *read_text(NSString *path) {
  NSError *error = nil;
  NSString *source = [NSString stringWithContentsOfFile:path
                                               encoding:NSUTF8StringEncoding
                                                  error:&error];
  if (source == nil) {
    fprintf(stderr, "failed to read %s: %s\n", path.UTF8String,
            error.localizedDescription.UTF8String);
    exit(1);
  }
  return source;
}

static id<MTLComputePipelineState>
load_compute_pipeline(id<MTLDevice> device, NSString *path) {
  NSError *error = nil;
  id<MTLLibrary> library =
      [device newLibraryWithSource:read_text(path) options:nil error:&error];
  if (library == nil) {
    fprintf(stderr, "failed to compile generated Mech Metal: %s\n",
            error.localizedDescription.UTF8String);
    exit(1);
  }
  id<MTLFunction> function = [library newFunctionWithName:@"mech_turn"];
  id<MTLComputePipelineState> pipeline =
      [device newComputePipelineStateWithFunction:function error:&error];
  if (pipeline == nil) {
    fprintf(stderr, "failed to create Mech compute pipeline: %s\n",
            error.localizedDescription.UTF8String);
    exit(1);
  }
  return pipeline;
}

static id<MTLRenderPipelineState>
load_render_pipeline(id<MTLDevice> device, NSString *path,
                     MTLPixelFormat format) {
  NSError *error = nil;
  id<MTLLibrary> library =
      [device newLibraryWithSource:read_text(path) options:nil error:&error];
  if (library == nil) {
    fprintf(stderr, "failed to compile render shim: %s\n",
            error.localizedDescription.UTF8String);
    exit(1);
  }
  MTLRenderPipelineDescriptor *descriptor =
      [[MTLRenderPipelineDescriptor alloc] init];
  descriptor.vertexFunction = [library newFunctionWithName:@"particle_vertex"];
  descriptor.fragmentFunction =
      [library newFunctionWithName:@"particle_fragment"];
  descriptor.colorAttachments[0].pixelFormat = format;
  descriptor.colorAttachments[0].blendingEnabled = YES;
  descriptor.colorAttachments[0].sourceRGBBlendFactor =
      MTLBlendFactorSourceAlpha;
  descriptor.colorAttachments[0].destinationRGBBlendFactor = MTLBlendFactorOne;
  descriptor.colorAttachments[0].sourceAlphaBlendFactor = MTLBlendFactorOne;
  descriptor.colorAttachments[0].destinationAlphaBlendFactor = MTLBlendFactorOne;
  id<MTLRenderPipelineState> pipeline =
      [device newRenderPipelineStateWithDescriptor:descriptor error:&error];
  if (pipeline == nil) {
    fprintf(stderr, "failed to create render pipeline: %s\n",
            error.localizedDescription.UTF8String);
    exit(1);
  }
  return pipeline;
}

static void write_ppm(NSString *path, const uint8_t *pixels, NSUInteger width,
                      NSUInteger height, NSUInteger bytes_per_row) {
  NSMutableData *output = [NSMutableData data];
  NSString *header =
      [NSString stringWithFormat:@"P6\n%lu %lu\n255\n", (unsigned long)width,
                                 (unsigned long)height];
  [output appendData:[header dataUsingEncoding:NSASCIIStringEncoding]];
  NSMutableData *row_data = [NSMutableData dataWithLength:width * 3];
  uint8_t *row_output = row_data.mutableBytes;
  for (NSUInteger y = 0; y < height; ++y) {
    const uint8_t *row = pixels + (height - 1 - y) * bytes_per_row;
    for (NSUInteger x = 0; x < width; ++x) {
      row_output[x * 3 + 0] = row[x * 4 + 2];
      row_output[x * 3 + 1] = row[x * 4 + 1];
      row_output[x * 3 + 2] = row[x * 4 + 0];
    }
    [output appendData:row_data];
  }
  if (![output writeToFile:path atomically:YES]) {
    fprintf(stderr, "failed to write capture to %s\n", path.UTF8String);
  } else {
    printf("capture: %s\n", path.UTF8String);
  }
}

@implementation ParticleRenderer {
  __weak ParticleView *_view;
  id<MTLDevice> _device;
  id<MTLCommandQueue> _queue;
  id<MTLComputePipelineState> _compute;
  id<MTLRenderPipelineState> _render;
  id<MTLBuffer> _state;
  dispatch_semaphore_t _frame_available;
  NSUInteger _lanes;
  NSUInteger _x_offset;
  NSUInteger _y_offset;
  NSUInteger _pointer_x_offset;
  NSUInteger _pointer_y_offset;
  NSUInteger _pointer_down_offset;
  NSUInteger _dt_offset;
  float _pointer_x;
  float _pointer_y;
  BOOL _pointer_down;
  float _camera_x;
  float _camera_y;
  float _zoom;
  CFTimeInterval _last_frame_time;
  CFTimeInterval _run_started;
  CFTimeInterval _stats_started;
  NSUInteger _stats_frames;
  NSUInteger _total_frames;
  NSUInteger _frame_limit;
  NSString *_capture_path;
}

- (instancetype)initWithView:(ParticleView *)view
                       device:(id<MTLDevice>)device
                  computePath:(NSString *)compute_path
                   renderPath:(NSString *)render_path
                 initialState:(NSString *)initial_state_path
                        lanes:(NSUInteger)lanes
                stateElements:(NSUInteger)state_elements
                  laneOffsets:(NSArray<NSNumber *> *)lane_offsets
                scalarOffsets:(NSArray<NSNumber *> *)scalar_offsets {
  self = [super init];
  if (self == nil) {
    return nil;
  }
  if (lane_offsets.count != 4 || scalar_offsets.count != 4) {
    fprintf(stderr, "the particle field requires four lane and four scalar states\n");
    exit(1);
  }

  NSData *initial = [NSData dataWithContentsOfFile:initial_state_path];
  NSUInteger expected_bytes = state_elements * sizeof(float);
  if (initial == nil || initial.length != expected_bytes) {
    fprintf(stderr, "initial state has %lu bytes; expected %lu\n",
            (unsigned long)initial.length, (unsigned long)expected_bytes);
    exit(1);
  }

  _view = view;
  _device = device;
  _queue = [device newCommandQueue];
  _compute = load_compute_pipeline(device, compute_path);
  _render = load_render_pipeline(device, render_path, view.colorPixelFormat);
  _state = [device newBufferWithBytes:initial.bytes
                               length:initial.length
                              options:MTLResourceStorageModeShared];
  _frame_available = dispatch_semaphore_create(1);
  _lanes = lanes;
  _x_offset = lane_offsets[0].unsignedIntegerValue;
  _y_offset = lane_offsets[1].unsignedIntegerValue;
  _pointer_x_offset = scalar_offsets[0].unsignedIntegerValue;
  _pointer_y_offset = scalar_offsets[1].unsignedIntegerValue;
  _pointer_down_offset = scalar_offsets[2].unsignedIntegerValue;
  _dt_offset = scalar_offsets[3].unsignedIntegerValue;
  _zoom = 0.92f;
  _stats_started = CACurrentMediaTime();

  const char *frame_limit = getenv("MECH_FRAMES");
  _frame_limit = frame_limit == NULL ? 0 : strtoull(frame_limit, NULL, 10);
  const char *capture_path = getenv("MECH_CAPTURE");
  if (capture_path != NULL && capture_path[0] != '\0') {
    _capture_path = [NSString stringWithUTF8String:capture_path];
  }

  printf("device: %s\n", device.name.UTF8String);
  printf("particles: %lu\n", (unsigned long)lanes);
  printf("resident state: %.3f MiB\n", expected_bytes / 1048576.0);
  printf("particle readback per frame: 0 bytes\n");
  return self;
}

- (void)updatePointer:(NSEvent *)event {
  ParticleView *view = _view;
  NSPoint point = [view convertPoint:event.locationInWindow fromView:nil];
  NSSize size = view.bounds.size;
  if (size.width <= 0.0 || size.height <= 0.0) {
    return;
  }
  float aspect = size.width / size.height;
  float normalized_x = (float)(point.x / size.width * 2.0 - 1.0);
  float normalized_y = (float)(point.y / size.height * 2.0 - 1.0);
  _pointer_x = _camera_x + normalized_x * aspect / _zoom;
  _pointer_y = _camera_y + normalized_y / _zoom;
}

- (void)pointerMoved:(NSEvent *)event {
  [self updatePointer:event];
}

- (void)pointerDown:(NSEvent *)event {
  _pointer_down = YES;
  [self updatePointer:event];
}

- (void)pointerUp:(NSEvent *)event {
  _pointer_down = NO;
  [self updatePointer:event];
}

- (void)pan:(NSEvent *)event {
  NSSize size = _view.bounds.size;
  if (size.height <= 0.0) {
    return;
  }
  float scale = 2.0f / ((float)size.height * _zoom);
  _camera_x -= (float)event.deltaX * scale;
  _camera_y += (float)event.deltaY * scale;
}

- (void)zoom:(NSEvent *)event {
  _zoom = fminf(8.0f, fmaxf(0.2f, _zoom * expf((float)event.scrollingDeltaY * 0.025f)));
  [self updatePointer:event];
}

- (void)mtkView:(MTKView *)view drawableSizeWillChange:(CGSize)size {
}

- (void)drawInMTKView:(MTKView *)view {
  dispatch_semaphore_wait(_frame_available, DISPATCH_TIME_FOREVER);
  @autoreleasepool {
    id<CAMetalDrawable> drawable = view.currentDrawable;
    MTLRenderPassDescriptor *render_pass = view.currentRenderPassDescriptor;
    if (drawable == nil || render_pass == nil) {
      dispatch_semaphore_signal(_frame_available);
      return;
    }

    CFTimeInterval now = CACurrentMediaTime();
    if (_run_started == 0.0) {
      _run_started = now;
    }
    float dt = _last_frame_time == 0.0
                   ? 1.0f / 60.0f
                   : (float)fmin(1.0 / 30.0, fmax(1.0 / 240.0, now - _last_frame_time));
    _last_frame_time = now;
    float *state = _state.contents;
    state[_pointer_x_offset] = _pointer_x;
    state[_pointer_y_offset] = _pointer_y;
    state[_pointer_down_offset] = _pointer_down ? 1.0f : 0.0f;
    state[_dt_offset] = dt;

    id<MTLCommandBuffer> command_buffer = [_queue commandBuffer];
    id<MTLComputeCommandEncoder> compute =
        [command_buffer computeCommandEncoder];
    [compute setComputePipelineState:_compute];
    [compute setBuffer:_state offset:0 atIndex:0];
    NSUInteger width = MIN((NSUInteger)256, _compute.maxTotalThreadsPerThreadgroup);
    [compute dispatchThreads:MTLSizeMake(_lanes, 1, 1)
        threadsPerThreadgroup:MTLSizeMake(width, 1, 1)];
    [compute endEncoding];

    CGSize drawable_size = view.drawableSize;
    RenderConfig config = {
        .x_offset = (uint32_t)_x_offset,
        .y_offset = (uint32_t)_y_offset,
        .camera_x = _camera_x,
        .camera_y = _camera_y,
        .zoom = _zoom,
        .aspect = drawable_size.height == 0.0
                      ? 1.0f
                      : (float)(drawable_size.width / drawable_size.height),
        .point_size = 2.2f,
        .padding = 0.0f,
    };
    render_pass.colorAttachments[0].clearColor =
        MTLClearColorMake(0.018, 0.025, 0.038, 1.0);
    id<MTLRenderCommandEncoder> render =
        [command_buffer renderCommandEncoderWithDescriptor:render_pass];
    [render setRenderPipelineState:_render];
    [render setVertexBuffer:_state offset:0 atIndex:0];
    [render setVertexBytes:&config length:sizeof(config) atIndex:1];
    [render drawPrimitives:MTLPrimitiveTypePoint vertexStart:0 vertexCount:_lanes];
    [render endEncoding];

    ++_total_frames;
    BOOL final_frame = _frame_limit > 0 && _total_frames >= _frame_limit;
    id<MTLBuffer> capture_buffer = nil;
    NSUInteger capture_width = 0;
    NSUInteger capture_height = 0;
    NSUInteger capture_bytes_per_row = 0;
    if (final_frame && _capture_path != nil) {
      capture_width = drawable.texture.width;
      capture_height = drawable.texture.height;
      capture_bytes_per_row = ((capture_width * 4 + 255) / 256) * 256;
      capture_buffer = [_device
          newBufferWithLength:capture_bytes_per_row * capture_height
                      options:MTLResourceStorageModeShared];
      id<MTLBlitCommandEncoder> blit = [command_buffer blitCommandEncoder];
      [blit copyFromTexture:drawable.texture
                sourceSlice:0
                sourceLevel:0
               sourceOrigin:MTLOriginMake(0, 0, 0)
                 sourceSize:MTLSizeMake(capture_width, capture_height, 1)
                   toBuffer:capture_buffer
          destinationOffset:0
     destinationBytesPerRow:capture_bytes_per_row
   destinationBytesPerImage:capture_bytes_per_row * capture_height];
      [blit endEncoding];
    }

    [command_buffer presentDrawable:drawable];
    NSString *capture_path = _capture_path;
    [command_buffer addCompletedHandler:^(id<MTLCommandBuffer> completed) {
      if (completed.status == MTLCommandBufferStatusError) {
        fprintf(stderr, "GPU frame failed: %s\n",
                completed.error.localizedDescription.UTF8String);
      }
      if (capture_buffer != nil) {
        write_ppm(capture_path, capture_buffer.contents, capture_width,
                  capture_height, capture_bytes_per_row);
      }
      dispatch_semaphore_signal(self->_frame_available);
      if (final_frame) {
        double elapsed = CACurrentMediaTime() - self->_run_started;
        double frame_rate = self->_total_frames / elapsed;
        printf("average frame rate: %.1f Hz\n", frame_rate);
        printf("average throughput: %.1f million particle updates/s\n",
               frame_rate * (double)self->_lanes / 1.0e6);
        dispatch_async(dispatch_get_main_queue(), ^{
          self->_view.paused = YES;
          [NSApp terminate:nil];
        });
      }
    }];
    [command_buffer commit];

    ++_stats_frames;
    CFTimeInterval stats_elapsed = now - _stats_started;
    if (stats_elapsed >= 1.0) {
      double fps = _stats_frames / stats_elapsed;
      double updates = fps * (double)_lanes / 1.0e6;
      view.window.title = [NSString
          stringWithFormat:@"Mech Metal Particle Field | %.1f Hz | %.1f M updates/s",
                           fps, updates];
      _stats_frames = 0;
      _stats_started = now;
    }
  }
}
@end

static NSArray<NSNumber *> *parse_offsets(const char *text) {
  NSString *value = [NSString stringWithUTF8String:text];
  NSMutableArray<NSNumber *> *result = [NSMutableArray array];
  for (NSString *part in [value componentsSeparatedByString:@","]) {
    if (part.length == 0) {
      continue;
    }
    NSScanner *scanner = [NSScanner scannerWithString:part];
    unsigned long long offset = 0;
    if (![scanner scanUnsignedLongLong:&offset] || !scanner.isAtEnd) {
      fprintf(stderr, "invalid state offset: %s\n", part.UTF8String);
      exit(2);
    }
    [result addObject:@((NSUInteger)offset)];
  }
  return result;
}

@interface ParticleAppDelegate : NSObject <NSApplicationDelegate>
- (instancetype)initWithArguments:(const char **)arguments;
@end

@implementation ParticleAppDelegate {
  NSArray<NSString *> *_paths;
  NSUInteger _lanes;
  NSUInteger _state_elements;
  NSArray<NSNumber *> *_lane_offsets;
  NSArray<NSNumber *> *_scalar_offsets;
  NSWindow *_window;
  ParticleRenderer *_renderer;
}

- (instancetype)initWithArguments:(const char **)arguments {
  self = [super init];
  if (self != nil) {
    _paths = @[
      [NSString stringWithUTF8String:arguments[1]],
      [NSString stringWithUTF8String:arguments[2]],
      [NSString stringWithUTF8String:arguments[3]],
    ];
    _lanes = strtoull(arguments[4], NULL, 10);
    _state_elements = strtoull(arguments[5], NULL, 10);
    _lane_offsets = parse_offsets(arguments[6]);
    _scalar_offsets = parse_offsets(arguments[7]);
  }
  return self;
}

- (void)applicationDidFinishLaunching:(NSNotification *)notification {
  id<MTLDevice> device = MTLCreateSystemDefaultDevice();
  if (device == nil) {
    fprintf(stderr, "no Metal device is available\n");
    [NSApp terminate:nil];
    return;
  }

  NSRect frame = NSMakeRect(0, 0, 1200, 760);
  _window = [[NSWindow alloc]
      initWithContentRect:frame
                styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
                          NSWindowStyleMaskResizable | NSWindowStyleMaskMiniaturizable
                  backing:NSBackingStoreBuffered
                    defer:NO];
  _window.title = @"Mech Metal Particle Field";
  _window.backgroundColor = NSColor.blackColor;
  [_window center];

  ParticleView *view = [[ParticleView alloc] initWithFrame:frame device:device];
  view.colorPixelFormat = MTLPixelFormatBGRA8Unorm;
  view.clearColor = MTLClearColorMake(0.018, 0.025, 0.038, 1.0);
  view.preferredFramesPerSecond = 60;
  view.enableSetNeedsDisplay = NO;
  view.paused = NO;
  view.framebufferOnly = NO;
  view.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;

  _renderer = [[ParticleRenderer alloc]
          initWithView:view
                 device:device
            computePath:_paths[0]
             renderPath:_paths[1]
           initialState:_paths[2]
                  lanes:_lanes
          stateElements:_state_elements
            laneOffsets:_lane_offsets
          scalarOffsets:_scalar_offsets];
  view.input_controller = _renderer;
  view.delegate = _renderer;
  _window.contentView = view;
  [_window makeFirstResponder:view];
  [_window makeKeyAndOrderFront:nil];
  [NSApp activateIgnoringOtherApps:YES];
}

- (BOOL)applicationShouldTerminateAfterLastWindowClosed:(NSApplication *)sender {
  return YES;
}
@end

int main(int argc, const char *argv[]) {
  @autoreleasepool {
    if (argc != 8) {
      fprintf(stderr,
              "usage: %s turn.metal render.metal initial.f32 lanes "
              "state-elements lane-offsets scalar-offsets\n",
              argv[0]);
      return 2;
    }
    [NSApplication sharedApplication];
    NSApp.activationPolicy = NSApplicationActivationPolicyRegular;
    ParticleAppDelegate *delegate =
        [[ParticleAppDelegate alloc] initWithArguments:argv];
    NSApp.delegate = delegate;
    [NSApp run];
  }
  return 0;
}
