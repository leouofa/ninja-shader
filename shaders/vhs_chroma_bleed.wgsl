// VHS Chroma Bleed / Analog Subcarrier Smear
// Globals and Params are auto-injected by ninja-shader

// RGB to YIQ conversion matrix
fn rgb2yiq(c: vec3<f32>) -> vec3<f32> {
    let y = dot(c, vec3<f32>(0.299, 0.587, 0.114));
    let i = dot(c, vec3<f32>(0.596, -0.274, -0.322));
    let q = dot(c, vec3<f32>(0.211, -0.523, 0.312));
    return vec3<f32>(y, i, q);
}

// YIQ to RGB conversion matrix
fn yiq2rgb(yiq: vec3<f32>) -> vec3<f32> {
    let r = yiq.x + 0.956 * yiq.y + 0.621 * yiq.z;
    let g = yiq.x - 0.272 * yiq.y - 0.647 * yiq.z;
    let b = yiq.x - 1.106 * yiq.y + 1.703 * yiq.z;
    return clamp(vec3<f32>(r, g, b), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sharp luminance (Y)
    let sharp_rgb = textureSampleLevel(input_tex, linear_sampler, in.uv, 0.0);
    let sharp_y = rgb2yiq(sharp_rgb.rgb).x;

    // Horizontal low-pass filtered chrominance (I & Q) with phase shift
    let chroma_uv = in.uv + vec2<f32>(params.chroma_shift, 0.0);
    let step_x = params.bleed_distance / 4.0;

    var i_accum = 0.0;
    var q_accum = 0.0;
    var weight_total = 0.0;

    for (var k = -4; k <= 4; k = k + 1) {
        let offset_x = f32(k) * step_x;
        let sample_uv = vec2<f32>(clamp(chroma_uv.x + offset_x, 0.0, 1.0), chroma_uv.y);
        let sample_col = textureSampleLevel(input_tex, linear_sampler, sample_uv, 0.0).rgb;
        let yiq = rgb2yiq(sample_col);

        let weight = exp(-f32(k * k) / 8.0);
        i_accum = i_accum + yiq.y * weight;
        q_accum = q_accum + yiq.z * weight;
        weight_total = weight_total + weight;
    }

    let smooth_i = (i_accum / weight_total) * params.saturation;
    let smooth_q = (q_accum / weight_total) * params.saturation;

    let final_rgb = yiq2rgb(vec3<f32>(sharp_y, smooth_i, smooth_q));
    return vec4<f32>(final_rgb, sharp_rgb.a);
}
