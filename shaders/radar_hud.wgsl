// Sci-Fi Radar HUD & Reticle Overlay
// Globals and Params are auto-injected by ninja-shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSampleLevel(input_tex, linear_sampler, in.uv, 0.0);

    let aspect = globals.resolution.x * globals.inv_resolution.y;
    let center_uv = (in.uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(aspect, 1.0);
    let dist = length(center_uv);
    let angle = atan2(center_uv.y, center_uv.x);

    var hud = 0.0;

    // Reticle rings
    let r1 = abs(dist - params.reticle_scale * 0.5);
    let r2 = abs(dist - params.reticle_scale);
    hud = hud + (1.0 - smoothstep(0.0, 0.003, r1)) * 0.7;
    hud = hud + (1.0 - smoothstep(0.0, 0.003, r2));

    // Crosshairs with center gap
    if (dist < params.reticle_scale * 1.3 && dist > 0.03) {
        let line_x = abs(center_uv.y);
        let line_y = abs(center_uv.x);
        hud = hud + (1.0 - smoothstep(0.0, 0.0015, line_x)) * 0.5;
        hud = hud + (1.0 - smoothstep(0.0, 0.0015, line_y)) * 0.5;
    }

    // Rotating radar beam
    let sweep_angle = fract(globals.time * params.sweep_speed / 6.28318) * 6.28318 - 3.14159;
    var d_angle = sweep_angle - angle;
    if (d_angle < 0.0) {
        d_angle = d_angle + 6.28318;
    }
    if (dist < params.reticle_scale) {
        let beam = exp(-d_angle * 3.5);
        hud = hud + beam * 0.45;
    }

    // Corner brackets
    let corner_uv = abs(in.uv - vec2<f32>(0.5, 0.5));
    if (corner_uv.x > 0.42 && corner_uv.y > 0.42) {
        let edge_x = abs(corner_uv.x - 0.46);
        let edge_y = abs(corner_uv.y - 0.46);
        let bracket = (1.0 - smoothstep(0.0, 0.003, edge_x)) + (1.0 - smoothstep(0.0, 0.003, edge_y));
        hud = hud + clamp(bracket, 0.0, 0.8);
    }

    let final_rgb = mix(base.rgb, params.hud_color, clamp(hud * params.hud_opacity, 0.0, 1.0));
    return vec4<f32>(final_rgb, base.a);
}
