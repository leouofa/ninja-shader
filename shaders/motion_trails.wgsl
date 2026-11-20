// Motion Trails using prev_frame_tex temporal feedback
// Globals and Params are auto-injected by ninja-shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let current = textureSample(input_tex, linear_sampler, in.uv);

    // On frame 0, no previous frame feedback exists yet
    if (globals.frame == 0u) {
        return current;
    }

    let previous = textureSample(prev_frame_tex, linear_sampler, in.uv);

    // Blend current frame with decaying previous frame
    let blended_rgb = max(current.rgb, previous.rgb * params.decay);
    return vec4<f32>(blended_rgb, current.a);
}
