// Vignette shader
// Globals and Params are auto-injected by ninja-shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(input_tex, linear_sampler, in.uv);

    // Centered coordinates scaled to aspect ratio
    let aspect = globals.resolution.x * globals.inv_resolution.y;
    let coord = (in.uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(aspect, 1.0);
    let dist = length(coord);

    let inner = max(params.radius - params.softness, 0.0);
    let outer = max(params.radius, inner + 0.001);
    let vignette = 1.0 - smoothstep(inner, outer, dist);
    let dark_factor = mix(1.0 - params.intensity, 1.0, vignette);

    let shaded_rgb = mix(params.tint, color.rgb, dark_factor);
    return vec4<f32>(shaded_rgb, color.a);
}
