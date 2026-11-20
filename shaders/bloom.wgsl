// Neon Bloom Shader
// Globals and Params are auto-injected by ninja-shader

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

fn extract_bright(c: vec3<f32>, threshold: f32, knee: f32) -> vec3<f32> {
    let brightness = luma(c);
    let soft = brightness - threshold + knee;
    let soft_clamped = clamp(soft, 0.0, 2.0 * knee);
    let weight = max(brightness - threshold, (soft_clamped * soft_clamped) / (4.0 * knee + 0.0001));
    return c * max(weight / max(brightness, 0.0001), 0.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSampleLevel(input_tex, linear_sampler, in.uv, 0.0);

    let px = globals.inv_resolution * params.radius;
    var bloom_accum = extract_bright(base.rgb, params.threshold, params.soft_knee) * 0.25;

    // 8-direction dual-ring kernel
    let offsets = array<vec2<f32>, 8>(
        vec2<f32>( 1.0,  0.0),
        vec2<f32>(-1.0,  0.0),
        vec2<f32>( 0.0,  1.0),
        vec2<f32>( 0.0, -1.0),
        vec2<f32>( 0.707,  0.707),
        vec2<f32>(-0.707,  0.707),
        vec2<f32>( 0.707, -0.707),
        vec2<f32>(-0.707, -0.707)
    );

    // Inner ring (scale 1.0)
    for (var i = 0; i < 8; i = i + 1) {
        let sample_col = textureSampleLevel(input_tex, linear_sampler, in.uv + offsets[i] * px, 0.0).rgb;
        bloom_accum = bloom_accum + extract_bright(sample_col, params.threshold, params.soft_knee) * 0.06;
    }

    // Outer ring (scale 2.2)
    for (var i = 0; i < 8; i = i + 1) {
        let sample_col = textureSampleLevel(input_tex, linear_sampler, in.uv + offsets[i] * px * 2.2, 0.0).rgb;
        bloom_accum = bloom_accum + extract_bright(sample_col, params.threshold, params.soft_knee) * 0.035;
    }

    let final_rgb = base.rgb + bloom_accum * params.intensity;
    return vec4<f32>(final_rgb, base.a);
}
