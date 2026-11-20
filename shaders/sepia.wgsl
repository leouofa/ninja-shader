// Fullscreen fragment shader
// Globals and Params are auto-injected if not explicitly declared

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(input_tex, linear_sampler, in.uv);
    color.r = color.r * params.strength;
    return color;
}
