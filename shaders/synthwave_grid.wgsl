// Synthwave / Outrun Perspective Grid
// Globals and Params are auto-injected by ninja-shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSampleLevel(input_tex, linear_sampler, in.uv, 0.0);

    // Grid appears below the horizon line
    if (in.uv.y < params.horizon) {
        // Horizon haze glow
        let dist_to_horizon = params.horizon - in.uv.y;
        let glow = exp(-dist_to_horizon * 18.0) * params.opacity * 0.4;
        let final_rgb = base.rgb + params.grid_color * glow;
        return vec4<f32>(final_rgb, base.a);
    }

    // Perspective projection onto flat ground plane
    let depth = (in.uv.y - params.horizon) / (1.0 - params.horizon + 0.001);
    let z = 1.0 / max(depth, 0.02);
    let x = (in.uv.x - 0.5) * z;

    // Scrolling floor coordinates
    let grid_uv = vec2<f32>(x * params.grid_density * 0.15, z * 2.0 - globals.time * params.scroll_speed);
    let grid_cell = abs(fract(grid_uv) - 0.5);
    let line_dist = min(grid_cell.x, grid_cell.y);

    let line_width = 0.06 * z;
    let grid_line = 1.0 - smoothstep(0.0, line_width, line_dist);

    // Fog fade towards horizon
    let fog = smoothstep(0.0, 0.35, depth);
    let grid_intensity = grid_line * fog * params.opacity;

    let final_rgb = mix(base.rgb, params.grid_color, grid_intensity);
    return vec4<f32>(final_rgb, base.a);
}
