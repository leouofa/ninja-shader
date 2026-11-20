// Neon Sobel Edge Detection
// Globals and Params are auto-injected by ninja-shader

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let px = globals.inv_resolution;
    let base = textureSampleLevel(input_tex, linear_sampler, in.uv, 0.0);

    // Sobel 3x3 kernel samples
    let s00 = luma(textureSampleLevel(input_tex, linear_sampler, in.uv + vec2<f32>(-px.x, -px.y), 0.0).rgb);
    let s01 = luma(textureSampleLevel(input_tex, linear_sampler, in.uv + vec2<f32>( 0.0,  -px.y), 0.0).rgb);
    let s02 = luma(textureSampleLevel(input_tex, linear_sampler, in.uv + vec2<f32>( px.x, -px.y), 0.0).rgb);
    let s10 = luma(textureSampleLevel(input_tex, linear_sampler, in.uv + vec2<f32>(-px.x,   0.0), 0.0).rgb);
    let s12 = luma(textureSampleLevel(input_tex, linear_sampler, in.uv + vec2<f32>( px.x,   0.0), 0.0).rgb);
    let s20 = luma(textureSampleLevel(input_tex, linear_sampler, in.uv + vec2<f32>(-px.x,  px.y), 0.0).rgb);
    let s21 = luma(textureSampleLevel(input_tex, linear_sampler, in.uv + vec2<f32>( 0.0,   px.y), 0.0).rgb);
    let s22 = luma(textureSampleLevel(input_tex, linear_sampler, in.uv + vec2<f32>( px.x,  px.y), 0.0).rgb);

    let gx = s02 + 2.0 * s12 + s22 - (s00 + 2.0 * s10 + s20);
    let gy = s20 + 2.0 * s21 + s22 - (s00 + 2.0 * s01 + s02);
    let edge = sqrt(gx * gx + gy * gy) * params.edge_strength;

    let pulse = 1.0 + sin(globals.time * params.pulse_speed) * 0.15;
    let edge_color = params.neon_color * edge * pulse;

    let final_rgb = base.rgb * params.base_mix + edge_color;
    return vec4<f32>(final_rgb, base.a);
}
