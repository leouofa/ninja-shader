// CRT Scanlines & Barrel Distortion
// Globals and Params are auto-injected by ninja-shader

fn curve_coords(uv: vec2<f32>, k: f32) -> vec2<f32> {
    var p = uv * 2.0 - 1.0;
    let offset = p.yx / vec2<f32>(k, k);
    p = p + p * offset * offset;
    return p * 0.5 + 0.5;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var curved_uv = in.uv;
    if (params.curvature > 0.001) {
        let k = 1.0 / params.curvature;
        curved_uv = curve_coords(in.uv, k);
    }

    // Black border outside curved screen
    if (curved_uv.x < 0.0 || curved_uv.x > 1.0 || curved_uv.y < 0.0 || curved_uv.y > 1.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    var color = textureSample(input_tex, linear_sampler, curved_uv);

    // Scanlines
    let scanline = sin((curved_uv.y * params.scanline_count + globals.time * 2.0) * 6.28318);
    let scanline_factor = 1.0 - (scanline * 0.5 + 0.5) * params.scanline_intensity;

    color = vec4<f32>(color.rgb * scanline_factor, color.a);
    return color;
}
