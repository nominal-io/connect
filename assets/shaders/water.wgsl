#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::forward_io::VertexOutput

@group(2) @binding(0)
var<uniform> time: f32;

// Noise function for water waves
fn noise(p: vec2<f32>) -> f32 {
    let pi = 3.14159265359;
    let dir1 = vec2<f32>(1.0, 0.8);
    let dir2 = vec2<f32>(-0.8, 1.0);
    let wave1 = sin(dot(dir1, p) * 0.6 + time);
    let wave2 = sin(dot(dir2, p) * 0.9 - time * 0.6);
    return (wave1 + wave2) * 0.5;
}

@vertex
fn vertex(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Add wave movement to the vertex position
    var modified_position = position;
    let wave_height = 0.2;
    modified_position.y += noise(position.xz) * wave_height;
    
    // Calculate world position
    out.world_position = vec4<f32>(modified_position, 1.0);
    out.position = mesh_view_bindings::view.clip_from_world * out.world_position;
    
    // Calculate modified normal for lighting
    let wave_normal = vec3<f32>(
        noise(position.xz + vec2<f32>(0.01, 0.0)) - noise(position.xz - vec2<f32>(0.01, 0.0)),
        1.0,
        noise(position.xz + vec2<f32>(0.0, 0.01)) - noise(position.xz - vec2<f32>(0.0, 0.01))
    );
    out.world_normal = normalize(wave_normal);
    
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let water_color = vec3<f32>(0.0, 0.3, 0.5);
    let highlight_color = vec3<f32>(0.3, 0.6, 0.8);
    
    // Basic lighting calculation
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 0.0));
    let normal = normalize(in.world_normal);
    let diffuse = max(dot(normal, light_dir), 0.0);
    
    // Add specular highlight
    let view_dir = normalize(mesh_view_bindings::view.world_position.xyz - in.world_position.xyz);
    let reflect_dir = reflect(-light_dir, normal);
    let specular = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
    
    // Mix colors based on lighting
    let final_color = mix(water_color, highlight_color, diffuse * 0.5 + specular);
    
    // Add transparency
    let alpha = 0.9;
    
    return vec4<f32>(final_color, alpha);
}
