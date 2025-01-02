#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::forward_io::VertexOutput

@group(2) @binding(0)
var<uniform> grid_scale: f32;

@group(2) @binding(1)
var<uniform> line_width: f32;

@vertex
fn vertex(
    @location(0) position: vec3<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.world_position = vec4<f32>(position, 1.0);
    out.position = mesh_view_bindings::view.clip_from_world * vec4<f32>(position, 1.0);
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Base colors
    let color1 = vec3<f32>(0.02, 0.02, 0.04); // Very dark blue-black
    let color2 = vec3<f32>(0.06, 0.08, 0.12); // Navy blue
    
    // Calculate distance from camera with adjusted fade distances
    let distance = length(in.world_position.xyz);
    let fade_start = 15.0;
    let fade_end = 70.0;
    let fade_factor = smoothstep(fade_start, fade_end, distance);
    
    // Calculate contrast reduction in distance
    let contrast_start = 20.0;
    let contrast_end = 60.0;
    let contrast_reduction = smoothstep(contrast_start, contrast_end, distance) * 0.7;
    
    // Calculate checkerboard pattern
    let coord = in.world_position.xyz.xz * (grid_scale * 0.5);
    let checker = floor(coord.x) + floor(coord.y);
    let is_even = fract(checker * 0.5) * 2.0;
    
    // Mix colors based on checkerboard pattern with reduced contrast in distance
    let mid_color = mix(color1, color2, 0.5);
    let pattern_color = select(color1, color2, is_even >= 1.0);
    let contrast_adjusted = mix(pattern_color, mid_color, contrast_reduction);
    
    // Fade to darker color in the distance
    let fade_color = color1;
    let color_with_fade = mix(contrast_adjusted, fade_color, fade_factor);
    
    return vec4<f32>(color_with_fade, 1.0);
} 