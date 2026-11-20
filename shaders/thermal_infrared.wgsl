// FLIR Thermal Infrared Heat Imaging
// Globals and Params are auto-injected by ninja-shader

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

// Ironbow thermal false-color palette
fn thermal_palette(t: f32) -> vec3<f32> {
    // 0.0: deep indigo, 0.25: purple, 0.5: crimson red, 0.75: bright orange/yellow, 1.0: white
    let r = smoothstep(0.35, 0.70, t) + smoothstep(0.85, 1.0, t) * 0.5;
    let g = smoothstep(0.55, 0.85, t) + smoothstep(0.90, 1.0, t) * 0.5;
    let b = (1.0 - smoothstep(0.15, 0.45, t)) * 0.6 + smoothstep(0.88, 1.0, t);
    return clamp(vec3<f32>(r, g, b), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSampleLevel(input_tex, linear_sampler, in.uv, 0.0);
    var heat = luma(base.rgb) * params.heat_gain;
    heat = clamp((heat - 0.5) * params.contrast + 0.5, 0.0, 1.0);

    // Isotherm lines
    if (params.isotherm_lines > 0.01) {
        let iso = sin(heat * 30.0 * 6.28318);
        let iso_edge = smoothstep(0.92, 1.0, iso) * params.isotherm_lines;
        heat = clamp(heat + iso_edge, 0.0, 1.0);
    }

    let thermal_rgb = thermal_palette(heat);
    return vec4<f32>(thermal_rgb, base.a);
}
