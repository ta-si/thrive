// assets/shaders/terrain.wgsl

// Common mesh/view bindings
#import bevy_pbr::mesh_functions

// IO for forward vs prepass (same entry names; types change under the flag)
#if PREPASS_PIPELINE
  #import bevy_pbr::prepass_io::{Vertex, VertexOutput, FragmentOutput}
#else
  #import bevy_pbr::forward_io::{Vertex, VertexOutput, FragmentOutput}
#endif

struct TileParams {
  tile_size: f32;
  height_scale: f32;
  texels_per_side: u32;
  _pad0: u32;
  tile_color: vec4<f32>;
};
@group(2) @binding(0) var<uniform> params: TileParams;

// Textures (no samplers; we use textureLoad)
@group(2) @binding(1) var height_tex: texture_2d<f32>;
@group(2) @binding(2) var normal_tex: texture_2d<f32>;

// Fetch height from R32Float at integer texel coords.
fn height_at(texel: vec2<i32>) -> f32 {
  return textureLoad(height_tex, texel, 0).r;
}

// Decode normal from RGBA8 into -1..1 range.
fn decode_normal(texel: vec2<i32>) -> vec3<f32> {
  let c = textureLoad(normal_tex, texel, 0).xyz;
  return normalize(c * 2.0 - vec3<f32>(1.0, 1.0, 1.0));
}

@vertex
fn vertex(in: Vertex) -> VertexOutput {
  var out: VertexOutput;

  // Base world position (flat plane mesh already placed in world by Transform)
  let world_from_local = mesh::get_world_from_local();
  let world_pos_flat = (world_from_local * vec4<f32>(in.position, 1.0)).xyz;

  // Compute texel coords from UV (0..1) * (N-1)
  let n_minus_1 = max(1u, params.texels_per_side - 1u);
  let uv = clamp(in.uv, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
  let texel = vec2<i32>(vec2<u32>(uv * f32(n_minus_1)));

  // Displace by height
  let h = height_at(texel) * params.height_scale;
  let displaced = vec3<f32>(world_pos_flat.x, world_pos_flat.y + h, world_pos_flat.z);

  // Output positions (clip + prev clip) and attributes
  out.position = mesh::get_clip_from_world() * vec4<f32>(displaced, 1.0);
  out.prev_clip_position = mesh::get_prev_clip_from_world() * vec4<f32>(displaced, 1.0);
  out.world_position = displaced;

  // World normal: prefer normal map’s Y-up if available; fall back to mesh normal
  let nmap = decode_normal(texel);
  let world_normal_from_local = mesh::get_world_normal_from_local();
  let mesh_world_n = normalize((world_normal_from_local * vec4<f32>(in.normal, 0.0)).xyz);
  out.world_normal = normalize(mix(mesh_world_n, nmap, 1.0));

  out.uv = in.uv;
  return out;
}

@fragment
fn fragment(_in: VertexOutput) -> FragmentOutput {
  var out: FragmentOutput;
  // For now, just output the per-tile color so we can see the grid works.
  out.color = params.tile_color;
  return out;
}
