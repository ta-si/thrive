// terrain_shader.wgsl
// ──────────────────────────────────────────────────────────────
//
// Combined WGSL for GPU‐driven terrain in Bevy 0.16.1, with holes, PBR lighting, and Bevy’s forward_io.
//
// Bindings (set 0):
//   ‒ binding 0 : Uniform<WorldSettings>        // tile_size, max_subdivisions, grid_dimensions
//   ‒ binding 1 : Texture2DArray<f32> heightTex  // R32Float, one slice per tile
//   ‒ binding 2 : Texture2DArray<vec4<u32>> splatTex // RGBA16Uint, one slice per tile
//   ‒ binding 3 : StorageBuffer<TileInfo[]>      // per‐instance tile data
//   ‒ binding 4 : Uniform<TerrainColor[]>        // palette: up to 256 colors
//   ‒ binding 5 : Uniform<CameraUBO>             // view_proj, camera_pos
//
// We import Bevy’s forward path I/O so that the fragment stage can hook into shadows, lights, etc.

#import bevy_pbr::forward_io

enable f16;

// ----------------------------------------------------------------
// 1) DATA STRUCTURES
// ----------------------------------------------------------------

struct WorldSettings {
    tile_size:        f32,
    max_subdivisions: u32,
    grid_dimensions:  vec2<u32>,
    _pad:             u32,
};

@group(0) @binding(0)
var<uniform> world: WorldSettings;

@group(0) @binding(1)
var heightTex: texture_2d_array<f32>;   // R32Float

@group(0) @binding(2)
var splatTex: texture_2d_array<vec4<u32>>; // RGBA16Uint

struct TileInfo {
    tile_coord:    vec2<u32>,  // (x, y) grid
    heightmap_idx: u32,        // index into heightTex array
    splatmap_idx:  u32,        // index into splatTex array
    _pad:          u32,
};

@group(0) @binding(3)
var<storage, read> tile_buffer: array<TileInfo>;

struct TerrainColor {
    albedo: vec3<f32>,
    _pad:   f32,
};

@group(0) @binding(4)
var<uniform> terrain_colors: array<TerrainColor>;

// Bevy’s forward pass I/O uniform
@group(0) @binding(5)
var<uniform> camera: CameraUBO;

// ----------------------------------------------------------------
// 2) VERTEX STAGE
// ----------------------------------------------------------------

struct VertexInput {
    @location(0) inPos:     vec3<f32>, // (x,z) ∈ [0..1], y=0
    @location(1) inUV:      vec2<f32>, // ∈ [0..1]
    @builtin(instance_index) instanceID: u32,
};

struct VSOutput {
    @builtin(position) Position: vec4<f32>,
    @location(0) vUV:          vec2<f32>,
    @location(1) vHeightIdx:   u32,
    @location(2) vSplatIdx:    u32,
    @location(3) vNormal:      vec3<f32>,
};

@vertex
fn vertex_main(input: VertexInput) -> VSOutput {
    var out: VSOutput;

    // 1) Fetch per‐tile instance data
    let info: TileInfo = tile_buffer[input.instanceID];
    let tc: vec2<f32> = vec2<f32>(info.tile_coord);
    let scale: f32 = world.tile_size;

    // 2) Compute world-space XZ
    let localX: f32 = input.inPos.x;
    let localZ: f32 = input.inPos.z;
    let worldX: f32 = tc.x * scale + localX * scale;
    let worldZ: f32 = tc.y * scale + localZ * scale;

    // 3) Sample height from heightTex (slice = heightmap_idx)
    let dims: vec2<f32> = vec2<f32>(textureDimensions(heightTex, 0).xy);
    let ix: i32 = i32(clamp(input.inUV.x * dims.x, 0.0, dims.x - 1.0));
    let iy: i32 = i32(clamp(input.inUV.y * dims.y, 0.0, dims.y - 1.0));
    let h0: f32 = textureLoad(
        heightTex,
        vec3<i32>(ix, iy, i32(info.heightmap_idx)),
        0
    ).r;

    // 4) Reconstruct normal via finite differences
    let du: f32 = 1.0 / dims.x;
    let dv: f32 = 1.0 / dims.y;
    let uv_u: vec2<f32> = clamp(input.inUV + vec2<f32>(du, 0.0), vec2<f32>(0.0), vec2<f32>(1.0));
    let uv_d: vec2<f32> = clamp(input.inUV + vec2<f32>(0.0, dv), vec2<f32>(0.0), vec2<f32>(1.0));

    let iu: i32 = i32(clamp(uv_u.x * dims.x, 0.0, dims.x - 1.0));
    let ju: i32 = i32(clamp(uv_u.y * dims.y, 0.0, dims.y - 1.0));
    let idv: i32 = i32(clamp(uv_d.x * dims.x, 0.0, dims.x - 1.0));
    let jdv: i32 = i32(clamp(uv_d.y * dims.y, 0.0, dims.y - 1.0));

    let h_u: f32 = textureLoad(
        heightTex,
        vec3<i32>(iu, ju, i32(info.heightmap_idx)),
        0
    ).r;
    let h_d: f32 = textureLoad(
        heightTex,
        vec3<i32>(idv, jdv, i32(info.heightmap_idx)),
        0
    ).r;

    let dh_dx: f32 = (h_u - h0) / du;
    let dh_dz: f32 = (h_d - h0) / dv;
    let normal: vec3<f32> = normalize(vec3<f32>(-dh_dx, 1.0, -dh_dz));

    // 5) Compute clip‐space position
    let worldPos: vec4<f32> = camera.view_proj * vec4<f32>(worldX, h0, worldZ, 1.0);
    out.Position = worldPos;
    out.vUV = input.inUV;
    out.vHeightIdx = info.heightmap_idx;
    out.vSplatIdx = info.splatmap_idx;
    out.vNormal = normal;
    return out;
}

// ----------------------------------------------------------------
// 3) GEOMETRY STAGE
// ----------------------------------------------------------------
//
// Discard triangles whose all 3 vertices’ splat‐IDs == 255 (hole).
// We still pass UV, slice indices, normals to FS.

@stage(geometry)
fn geometry_main(
    @builtin(position) inPos:      array<vec4<f32>, 3>,
    @location(0) vUV:              array<vec2<f32>, 3>,
    @location(1) vHeightIdx:       array<u32, 3>,
    @location(2) vSplatIdx:        array<u32, 3>,
    @location(3) vNormal:          array<vec3<f32>, 3>
) {
    // Check all 3 vertices:
    for(var i: u32 = 0u; i < 3u; i = i + 1u) {
        let uv: vec2<f32> = vUV[i];
        let slice: u32 = vSplatIdx[i];
        let dims: vec2<f32> = vec2<f32>(textureDimensions(splatTex, 0).xy);
        let sx: i32 = i32(clamp(uv.x * dims.x, 0.0, dims.x - 1.0));
        let sy: i32 = i32(clamp(uv.y * dims.y, 0.0, dims.y - 1.0));
        let spl: vec4<u32> = textureLoad(
            splatTex,
            vec3<i32>(sx, sy, i32(slice)),
            0
        );
        let id_r: u32 = (spl.r >> 8u) & 0xFFu;
        let id_g: u32 = (spl.g >> 8u) & 0xFFu;
        let id_b: u32 = (spl.b >> 8u) & 0xFFu;
        let id_a: u32 = (spl.a >> 8u) & 0xFFu;
        if (id_r != 255u || id_g != 255u || id_b != 255u || id_a != 255u) {
            // This vertex is not fully hole → keep triangle
            break;
        }
        // If i==2 and still all channels==255 → discard
        if (i == 2u
            && id_r == 255u && id_g == 255u && id_b == 255u && id_a == 255u) {
            return; // drop tri
        }
    }
    // Otherwise, emit:
    for(var i: u32 = 0u; i < 3u; i = i + 1u) {
        SetBuiltinPosition(inPos[i]);
        SetLocation(0, vUV[i]);
        SetLocation(1, vHeightIdx[i]);
        SetLocation(2, vSplatIdx[i]);
        SetLocation(3, vNormal[i]);
        EmitVertex();
    }
    EndPrimitive();
}

// ----------------------------------------------------------------
// 4) FRAGMENT STAGE
// ----------------------------------------------------------------
//
// Blend up to 4 materials by weight; use Bevy’s PBR forward_io for lighting.

struct FSInput {
    @location(0) fUV:        vec2<f32>,
    @location(1) fHeightIdx: u32,
    @location(2) fSplatIdx:  u32,
    @location(3) fNormal:    vec3<f32>,
    @builtin(position) fragPos: vec4<f32>,
};

@fragment
fn fragment_main(input: FSInput) -> @location(0) vec4<f32> {
    // 1) Sample splatmap
    let dims: vec2<f32> = vec2<f32>(textureDimensions(splatTex, 0).xy);
    let sx: i32 = i32(clamp(input.fUV.x * dims.x, 0.0, dims.x - 1.0));
    let sy: i32 = i32(clamp(input.fUV.y * dims.y, 0.0, dims.y - 1.0));
    let slice: i32 = i32(input.fSplatIdx);
    let spl: vec4<u32> = textureLoad(
        splatTex,
        vec3<i32>(sx, sy, slice),
        0
    );

    // 2) Unpack
    var accum_albedo: vec3<f32> = vec3<f32>(0.0);
    var total_weight: f32 = 0.0;

    // R channel
    let id_r: u32 = (spl.r >> 8u) & 0xFFu;
    let w_r: f32 = f32(spl.r & 0xFFu) / 255.0;
    if (id_r != 255u && w_r > 0.0) {
        accum_albedo += terrain_colors[id_r].albedo * w_r;
        total_weight += w_r;
    }
    // G channel
    let id_g: u32 = (spl.g >> 8u) & 0xFFu;
    let w_g: f32 = f32(spl.g & 0xFFu) / 255.0;
    if (id_g != 255u && w_g > 0.0) {
        accum_albedo += terrain_colors[id_g].albedo * w_g;
        total_weight += w_g;
    }
    // B channel
    let id_b: u32 = (spl.b >> 8u) & 0xFFu;
    let w_b: f32 = f32(spl.b & 0xFFu) / 255.0;
    if (id_b != 255u && w_b > 0.0) {
        accum_albedo += terrain_colors[id_b].albedo * w_b;
        total_weight += w_b;
    }
    // A channel
    let id_a: u32 = (spl.a >> 8u) & 0xFFu;
    let w_a: f32 = f32(spl.a & 0xFFu) / 255.0;
    if (id_a != 255u && w_a > 0.0) {
        accum_albedo += terrain_colors[id_a].albedo * w_a;
        total_weight += w_a;
    }

    // 3) Normalize
    var final_albedo: vec3<f32> = vec3<f32>(0.10, 0.60, 0.10);
    if (total_weight > 0.0) {
        final_albedo = accum_albedo / total_weight;
    }

    // 4) Build PBR inputs & call Bevy’s forward shading helper
    var pbr: PBRData;
    pbr.base_color = final_albedo;
    pbr.emissive = vec3<f32>(0.0);
    pbr.roughness = 0.8;
    pbr.metallic = 0.0;
    pbr.normal = normalize(input.fNormal);
    pbr.world_pos = input.fragPos.xyz;
    pbr.view_pos = camera.camera_pos;

    // Evaluate direct + indirect lighting (forward_io writes to fragment_out)
    var fragment_out: FragmentOutput;
    forward_fragment(pbr, fragment_out);
    return fragment_out.color;
}
