// RGB Wave Distortion
// Globals and Params are auto-injected by ninja-shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = globals.time * params.speed;
    let base_y = in.uv.y * params.frequency;

    let wave_r = sin(base_y + t) * params.amplitude;
    let wave_g = sin(base_y + t + params.rgb_phase) * params.amplitude;
    let wave_b = sin(base_y + t + params.rgb_phase * 2.0) * params.amplitude;

    let uv_r = vec2<f32>(fract(in.uv.x + wave_r), in.uv.y);
    let uv_g = vec2<f32>(fract(in.uv.x + wave_g), in.uv.y);
    let uv_b = vec2<f32>(fract(in.uv.x + wave_b), in.uv.y);

    let r = textureSampleLevel(input_tex, linear_sampler, uv_r, 0.0).r;
    let g = textureSampleLevel(input_tex, linear_sampler, uv_g, 0.0).g;
    let b = textureSampleLevel(input_tex, linear_sampler, uv_b, 0.0).b;
    let a = textureSampleLevel(input_tex, linear_sampler, in.uv, 0.0).a;

    return vec4<f32>(r, g, b, a);
}
