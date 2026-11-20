// Directional Pixel Sorting Simulation
// Globals and Params are auto-injected by ninja-shader

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSampleLevel(input_tex, linear_sampler, in.uv, 0.0);
    let base_lum = luma(base_color.rgb);

    // Only sort within threshold interval
    if (base_lum < params.threshold_low || base_lum > params.threshold_high) {
        return base_color;
    }

    let is_horizontal = params.direction > 0.5;
    let dir_vec = select(vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), is_horizontal);

    var brightest = base_color;
    var max_lum = base_lum;

    // Search along directional span for sorted color
    let steps = 8;
    for (var i = 1; i <= steps; i = i + 1) {
        let step_offset = f32(i) / f32(steps) * params.span_length;
        let sample_uv = in.uv - dir_vec * step_offset;

        if (sample_uv.x >= 0.0 && sample_uv.x <= 1.0 && sample_uv.y >= 0.0 && sample_uv.y <= 1.0) {
            let col = textureSampleLevel(input_tex, linear_sampler, sample_uv, 0.0);
            let lum = luma(col.rgb);
            if (lum >= params.threshold_low && lum <= params.threshold_high && lum > max_lum) {
                max_lum = lum;
                brightest = col;
            }
        }
    }

    return brightest;
}
