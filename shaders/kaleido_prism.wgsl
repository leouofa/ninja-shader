// Kaleidoscope Prism Shader
// Globals and Params are auto-injected by ninja-shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let aspect = globals.resolution.x * globals.inv_resolution.y;
    var p = (in.uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(aspect, 1.0) / max(params.zoom, 0.01);

    var r = length(p);
    var a = atan2(p.y, p.x) + globals.time * params.rotation_speed;

    let tau = 6.2831853;
    let n = max(floor(params.segments), 2.0);
    let segment_angle = tau / n;

    // Fold angle into kaleidoscope wedge
    a = (a % segment_angle + segment_angle) % segment_angle;
    a = abs(a - segment_angle * 0.5);

    // Reconstruct Cartesian UV
    let folded_coord = vec2<f32>(cos(a), sin(a)) * r;
    var sample_uv = folded_coord / vec2<f32>(aspect, 1.0) + vec2<f32>(0.5, 0.5);
    sample_uv = clamp(sample_uv, vec2<f32>(0.0), vec2<f32>(1.0));

    let prism_color = textureSampleLevel(input_tex, linear_sampler, sample_uv, 0.0);
    let base_color = textureSampleLevel(input_tex, linear_sampler, in.uv, 0.0);

    let final_rgb = mix(base_color.rgb, prism_color.rgb, params.mix_amount);
    return vec4<f32>(final_rgb, base_color.a);
}
