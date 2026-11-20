// Pixelate / Bit-Depth Quantization
// Globals and Params are auto-injected by ninja-shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let px_coord = in.uv * globals.resolution;
    let block = max(params.pixel_size, 1.0);
    let cell_coord = floor(px_coord / block);
    let quantized_uv = (cell_coord * block + block * 0.5) * globals.inv_resolution;

    var color = textureSampleLevel(input_tex, linear_sampler, clamp(quantized_uv, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0);

    // Color bit-depth reduction
    if (params.color_levels < 63.0) {
        let levels = max(params.color_levels, 2.0);
        color = vec4<f32>(floor(color.rgb * levels) / (levels - 1.0), color.a);
    }

    // Sub-pixel grid lines
    if (params.grid_line > 0.01 && block >= 3.0) {
        let local_px = fract(px_coord / block);
        let border_dist = min(min(local_px.x, 1.0 - local_px.x), min(local_px.y, 1.0 - local_px.y)) * block;
        let line_factor = 1.0 - (1.0 - smoothstep(0.0, 1.0, border_dist)) * params.grid_line;
        color = vec4<f32>(color.rgb * line_factor, color.a);
    }

    return color;
}
