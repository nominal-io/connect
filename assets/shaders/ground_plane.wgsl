#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_types

struct GroundPlaneUniforms {
    scale: f32,
    color1: vec4<f32>,
    color2: vec4<f32>,
    _padding: vec2<f32>,
};

@group(1) @binding(0)
var<uniform> material: GroundPlaneUniforms;

#ifdef PER_OBJECT_BUFFER_BATCH_SIZE
@group(2) @binding(0) 
var<uniform> mesh: array<mesh_types::Mesh, #{PER_OBJECT_BUFFER_BATCH_SIZE}u>;
#else
@group(2) @binding(0) 
var<storage> mesh: array<mesh_types::Mesh>;
#endif

// Noise functions
fn permute3(x: vec3<f32>) -> vec3<f32> {
    return (((x * 34.0) + 1.0) * x) % vec3<f32>(289.0);
}

fn snoise2(v: vec2<f32>) -> f32 {
    let C = vec4<f32>(0.211324865405187, 0.366025403784439, -0.577350269189626, 0.024390243902439);
    let i_pos = floor(v + dot(v, C.yy));
    let x0 = v - i_pos + dot(i_pos, C.xx);
    var i1: vec2<f32>;
    if (x0.x > x0.y) {
        i1 = vec2<f32>(1.0, 0.0);
    } else {
        i1 = vec2<f32>(0.0, 1.0);
    }
    let x1 = x0.xy + C.xx - i1;
    let x2 = x0.xy + C.zz;
    let grid_pos = i_pos % vec2<f32>(289.0);
    let p = permute3(permute3(grid_pos.y + vec3<f32>(0.0, i1.y, 1.0)) + grid_pos.x + vec3<f32>(0.0, i1.x, 1.0));
    var m = max(0.5 - vec3<f32>(dot(x0, x0), dot(x1, x1), dot(x2, x2)), vec3<f32>(0.0));
    m = m * m;
    m = m * m;
    let x = 2.0 * fract(p * C.www) - 1.0;
    let h = abs(x) - 0.5;
    let ox = floor(x + 0.5);
    let a0 = x - ox;
    m = m * (1.8 - 1.2 * a0 * a0);
    let g = vec3<f32>(a0.x * x0.x + h.x * x0.y, a0.yz * vec2<f32>(x1.x, x2.x) + h.yz * vec2<f32>(x1.y, x2.y));
    return 130.0 * dot(m, g);
}

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    
    let world_position = mesh[0].model * vec4<f32>(vertex.position, 1.0);
    
    out.world_position = world_position;
    out.clip_position = view.view_proj * world_position;
    out.world_normal = (mesh[0].inverse_transpose_model * vec4<f32>(vertex.normal, 0.0)).xyz;
    out.uv = vertex.uv;
    
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.world_position.xz * material.scale;
    let noise = snoise2(uv) * 0.5 + 0.5;
    
    // Mix colors based on noise
    let color = mix(material.color1, material.color2, noise);
    
    // Add distance fade
    let fade = 1.0 - smoothstep(0.0, 100.0, length(in.world_position.xz));
    let final_color = mix(vec4<f32>(0.8, 0.8, 0.8, 1.0), color, fade);
    
    return final_color;
}