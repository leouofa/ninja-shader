// Custom Texture Overlay
// Globals, Params, and custom texture declarations are auto-injected by ninja-shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSample(input_tex, linear_sampler, in.uv);
    let overlay_coord = fract(in.uv * params.uv_scale);
    let overlay = textureSample(overlay_tex, linear_sampler, overlay_coord);

    // Multiply / overlay blend
    let blended_rgb = mix(base.rgb, base.rgb * overlay.rgb, params.opacity);
    return vec4<f32>(blended_rgb, base.a);
}
