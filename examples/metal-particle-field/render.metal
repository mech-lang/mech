#include <metal_stdlib>
using namespace metal;

struct RenderConfig {
  uint x_offset;
  uint y_offset;
  float camera_x;
  float camera_y;
  float zoom;
  float aspect;
  float point_size;
  float padding;
};

struct ParticleVertex {
  float4 position [[position]];
  float point_size [[point_size]];
  float4 color;
};

vertex ParticleVertex particle_vertex(
    uint lane [[vertex_id]],
    const device float *state [[buffer(0)]],
    constant RenderConfig &config [[buffer(1)]]) {
  float x = (state[config.x_offset + lane] - config.camera_x) * config.zoom;
  float y = (state[config.y_offset + lane] - config.camera_y) * config.zoom;
  float speed_tint = fract(float(lane) * 0.61803398875);

  ParticleVertex output;
  output.position = float4(x / config.aspect, y, 0.0, 1.0);
  output.point_size = config.point_size;
  output.color = mix(float4(0.20, 0.78, 0.96, 0.52),
                     float4(0.96, 0.46, 0.35, 0.70), speed_tint);
  return output;
}

fragment float4 particle_fragment(
    ParticleVertex input [[stage_in]],
    float2 point [[point_coord]]) {
  float distance_from_center = length(point - 0.5);
  float alpha = smoothstep(0.5, 0.18, distance_from_center);
  return float4(input.color.rgb, input.color.a * alpha);
}
