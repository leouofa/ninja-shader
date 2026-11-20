// Halation Shader
// Globals and Params are auto-injected by ninja-shader

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSampleLevel(input_tex, linear_sampler, in.uv, 0.0);
    let px = globals.inv_resolution * params.radius;

    // 4 diagonal taps to detect high-contrast edge spread
    let taps = array<vec2<f32>, 4>(
        vec2<f32>( 0.707,  0.707),
        vec2<f32>(-0.707,  0.707),
        vec2<f32>( 0.707, -0.707),
        vec2<f32>(-0.707, -0.707)
    );

    var scatter = 0.0;
    for (var i = 0; i < 4; i = i + 1) {
        let col = textureSampleLevel(input_tex, linear_sampler, in.uv + taps[i] * px, 0.0).rgb;
        let lum = luma(col);
        scatter = scatter + max(lum - params.threshold, 0.0);
    }
    scatter = (scatter * 0.25) / (1.0 - params.threshold + 0.001);

    // Warm halation additive tint
    let halation_rgb = params.tint * scatter * params.strength;
    let final_rgb = base.rgb + halation_rgb;

    return vec4<f32>(final_rgb, base.a);
}
