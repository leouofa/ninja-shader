// Datamosh / I-Frame Drop Simulation
// Uses prev_frame_tex temporal feedback
// Globals and Params are auto-injected by ninja-shader

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let curr = textureSampleLevel(input_tex, linear_sampler, in.uv, 0.0);
    if (globals.frame == 0u) {
        return curr;
    }

    // Macroblock quantization
    let block_uv_scale = globals.resolution / max(params.block_size, 1.0);
    let block_uv = (floor(in.uv * block_uv_scale) + 0.5) / block_uv_scale;

    // Estimate optical gradient vector from block neighborhood
    let px = globals.inv_resolution.x * params.block_size;
    let py = globals.inv_resolution.y * params.block_size;

    let c_right = textureSampleLevel(input_tex, linear_sampler, block_uv + vec2<f32>(px, 0.0), 0.0).rgb;
    let c_left  = textureSampleLevel(input_tex, linear_sampler, block_uv - vec2<f32>(px, 0.0), 0.0).rgb;
    let c_up    = textureSampleLevel(input_tex, linear_sampler, block_uv + vec2<f32>(0.0, py), 0.0).rgb;
    let c_down  = textureSampleLevel(input_tex, linear_sampler, block_uv - vec2<f32>(0.0, py), 0.0).rgb;

    let grad_x = luma(c_right) - luma(c_left);
    let grad_y = luma(c_up) - luma(c_down);
    let motion = vec2<f32>(grad_x, grad_y) * params.motion_offset;

    // Sample previous frame at motion-displaced coordinate
    let prev_uv = clamp(in.uv - motion, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
    let prev = textureSampleLevel(prev_frame_tex, linear_sampler, prev_uv, 0.0);

    let diff = length(curr.rgb - prev.rgb);
    let mosh_factor = select(params.feedback, 0.0, diff > params.threshold);

    let result_rgb = mix(curr.rgb, prev.rgb, mosh_factor);
    return vec4<f32>(result_rgb, curr.a);
}
