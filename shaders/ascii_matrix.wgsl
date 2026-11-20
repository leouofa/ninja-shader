// Digital Rain & Matrix Glyph Quantization
// Globals and Params are auto-injected by ninja-shader

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let px_coord = in.uv * globals.resolution;
    let cell_coord = floor(px_coord / max(params.cell_size, 1.0));
    let local_uv = fract(px_coord / max(params.cell_size, 1.0));

    // Sample video luminance at cell center
    let center_uv = (cell_coord * params.cell_size + params.cell_size * 0.5) * globals.inv_resolution;
    let base = textureSampleLevel(input_tex, linear_sampler, clamp(center_uv, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0);
    let lum = luma(base.rgb);

    // Procedural pseudo-glyph bitmap generation
    let glyph_seed = hash12(cell_coord + vec2<f32>(0.0, floor(globals.time * 4.0)));
    let glyph_bits = floor(local_uv * 3.0);
    let dot_active = hash12(vec2<f32>(glyph_seed * 17.0 + glyph_bits.x, glyph_bits.y));

    // Density modulation by luminance: higher lum activates more cell dots
    let threshold = 1.0 - lum;
    var glyph = 0.0;
    if (dot_active > threshold) {
        // Soft round character sub-pixels
        let sub_dist = length(local_uv - (glyph_bits + 0.5) / 3.0);
        glyph = 1.0 - smoothstep(0.08, 0.16, sub_dist);
    }

    // Downward falling digital rain brightness trail
    let rain_trail = fract(cell_coord.y * 0.05 - globals.time * params.rain_speed * 0.1 + hash12(vec2<f32>(cell_coord.x, 1.0)));
    let rain_boost = pow(rain_trail, 4.0) * 1.5;

    let total_intensity = clamp((glyph * 0.8 + rain_boost * 0.4) * lum * 1.8, 0.0, 1.5);
    let matrix_rgb = mix(params.glyph_color, base.rgb, params.color_mix) * total_intensity;

    return vec4<f32>(matrix_rgb, base.a);
}
