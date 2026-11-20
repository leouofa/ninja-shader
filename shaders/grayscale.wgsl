// Grayscale shader
// Globals and Params are auto-injected by ninja-shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(input_tex, linear_sampler, in.uv);
    // Standard Rec. 709 luminance weights
    let luma = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let gray = vec3<f32>(luma);
    let final_rgb = mix(color.rgb, gray, params.strength);
    return vec4<f32>(final_rgb, color.a);
}
