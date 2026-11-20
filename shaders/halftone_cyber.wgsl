// Cyber Halftone / Dot-Matrix Screen
// Globals and Params are auto-injected by ninja-shader

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

fn rotate(p: vec2<f32>, a: f32) -> vec2<f32> {
    let s = sin(a);
    let c = cos(a);
    return vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSampleLevel(input_tex, linear_sampler, in.uv, 0.0);
    let lum = luma(base.rgb);

    let aspect = globals.resolution.x * globals.inv_resolution.y;
    let centered_uv = (in.uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(aspect, 1.0);
    let rot_uv = rotate(centered_uv, params.angle);

    let grid = rot_uv * params.dot_scale;
    let cell_pos = fract(grid) - 0.5;
    let dist = length(cell_pos);

    // Target dot radius scales with luminance & contrast
    let radius = clamp(pow(lum, 1.0 / max(params.contrast, 0.1)) * 0.7, 0.0, 0.7);
    let dot_mask = 1.0 - smoothstep(radius - 0.08, radius + 0.08, dist);

    let halftone_rgb = base.rgb * dot_mask * 1.3;
    let final_rgb = mix(base.rgb, halftone_rgb, params.blend);

    return vec4<f32>(final_rgb, base.a);
}
