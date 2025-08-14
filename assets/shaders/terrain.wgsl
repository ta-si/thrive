#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings
#import bevy_pbr::mesh_functions
#import bevy_pbr::forward_io::{Vertex, VertexOutput, FragmentOutput}

struct TileParams {
  tile_size: f32;
  height_scale: f32;
  texels_per_side: u32;
  _pad0: u32;
  tile_color: vec4<f32>;
};

@group(2) @binding(0) var<uniform> params: TileParams;
@group(2) @binding(1) var height_tex: texture_2d<f32>;
@group(2) @binding(2) var normal_tex: texture_2d<f32>; // unused for now

fn height_at_uv(uv: vec2<f32>) -> f32 {
  let N = f32(params.texels_per_side);
  let x = i32(clamp(floor(uv.x * (N - 1.0)), 0.0, N - 1.0));
  let y = i32(clamp(floor(uv.y * (N - 1.0)), 0.0, N - 1.0));
  return textureLoad(height_tex, vec2<i32>(x, y), 0).r;
}

@vertex
fn vertex(in: Vertex) -> VertexOutput {
  var out: VertexOutput;

  let h = height_at_uv(in.uv) * params.height_scale;

  let world_from_local = mesh::get_world_from_local();
  let local_pos  = vec4<f32>(in.position.x, in.position.y + h, in.position.z, 1.0);
  let world_pos  = (world_from_local * local_pos).xyz;

  out.position           = mesh::get_clip_from_world() * vec4<f32>(world_pos, 1.0);
  out.prev_clip_position = mesh::get_prev_clip_from_world() * vec4<f32>(world_pos, 1.0);
  out.world_position     = world_pos;
  out.world_normal       = normalize((mesh::get_inverse_transpose_model() * vec4<f32>(in.normal, 0.0)).xyz);
  out.uv                 = in.uv;
  return out;
}

@fragment
fn fragment(_in: VertexOutput) -> FragmentOutput {
  var out: FragmentOutput;
  out.color = params.tile_color; // obvious solid tiles while validating
  return out;
}
