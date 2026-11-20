// Hologram Projection Shader
// Globals and Params are auto-injected by ninja-shader

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Horizontal sync jitter
    let jitter_noise = sin(in.uv.y * 120.0 + globals.time * params.flicker_speed);
    let uv_x = in.uv.x + jitter_noise * params.jitter;
    let sample_uv = vec2<f32>(clamp(uv_x, 0.0, 1.0), in.uv.y);

    let base = textureSampleLevel(input_tex, linear_sampler, sample_uv, 0.0);
    let lum = luma(base.rgb);

    // Fine holographic scanlines
    let scanlines = sin(in.uv.y * params.scanline_density * 6.28318) * 0.5 + 0.5;

    // Rolling vertical sync bar
    let roll = fract(in.uv.y * 1.5 - globals.time * 0.8);
    let sync_bar = smoothstep(0.0, 0.12, roll) * (1.0 - smoothstep(0.12, 0.24, roll)) * 0.4;

    // Random brightness flicker
    let flicker = 0.9 + 0.1 * sin(globals.time * params.flicker_speed * 1.73);

    // Holographic emission composite
    let holo_intensity = (lum * 0.75 + scanlines * 0.25 + sync_bar) * flicker;
    let holo_rgb = params.tint * holo_intensity;

    return vec4<f32>(holo_rgb, base.a);
}
