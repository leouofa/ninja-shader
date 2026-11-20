// Cyberpunk Duotone / Split Toning
// Globals and Params are auto-injected by ninja-shader

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSampleLevel(input_tex, linear_sampler, in.uv, 0.0);
    var lum = luma(base.rgb);

    // S-curve contrast
    lum = clamp((lum - 0.5) * params.contrast + 0.5, 0.0, 1.0);
    lum = smoothstep(0.0, 1.0, lum);

    // Map luminance between shadow and highlight tones
    let duotone_rgb = mix(params.shadow_color, params.highlight_color, lum);
    let final_rgb = mix(base.rgb, duotone_rgb, params.mix_amount);

    return vec4<f32>(final_rgb, base.a);
}
