// Tactical Night Vision Goggles
// Globals and Params are auto-injected by ninja-shader

fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSampleLevel(input_tex, linear_sampler, in.uv, 0.0);

    // Night vision relies heavily on green & luminance channels
    let luma = dot(base.rgb, vec3<f32>(0.2, 0.7, 0.1));
    let amplified = pow(clamp(luma * params.gain, 0.0, 1.0), 0.85);

    // Scintillation noise
    let noise = (hash12(in.uv * globals.resolution + vec2<f32>(globals.time * 24.0, globals.time * 37.0)) - 0.5) * params.noise_amount;

    // Circular tube optics mask
    let aspect = globals.resolution.x * globals.inv_resolution.y;
    let d = length((in.uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(aspect, 1.0));
    let mask = 1.0 - smoothstep(0.45, 0.65, d) * params.tube_vignette;

    let final_lum = clamp(amplified + noise, 0.0, 1.0) * mask;
    let nvg_rgb = params.phosphor_tint * final_lum;

    return vec4<f32>(nvg_rgb, base.a);
}
