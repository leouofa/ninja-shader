// Analog VHS Tape Tracking & Head-Switching Noise
// Globals and Params are auto-injected by ninja-shader

fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = in.uv;

    // Line sync jitter
    let line_noise = (hash12(vec2<f32>(floor(in.uv.y * globals.resolution.y), globals.time * 60.0)) - 0.5) * 2.0;
    uv.x = uv.x + line_noise * params.line_jitter;

    // Rolling tracking bar
    let track_pos = fract(globals.time * params.scroll_speed);
    let dist_to_track = abs(in.uv.y - track_pos);
    if (dist_to_track < 0.06) {
        let tear = (1.0 - dist_to_track / 0.06);
        let track_noise = (hash12(vec2<f32>(floor(in.uv.y * 80.0), globals.time * 30.0)) - 0.5);
        uv.x = uv.x + track_noise * tear * params.bar_intensity * 0.12;
    }

    // Bottom head-switching noise
    let is_head_switch = in.uv.y > (1.0 - params.head_switch_height);
    if (is_head_switch) {
        let head_skew = (in.uv.y - (1.0 - params.head_switch_height)) / params.head_switch_height;
        let skew_noise = hash12(vec2<f32>(floor(in.uv.y * globals.resolution.y), globals.time * 45.0));
        uv.x = uv.x + (skew_noise - 0.5) * head_skew * 0.25;
    }

    uv = vec2<f32>(clamp(uv.x, 0.0, 1.0), clamp(uv.y, 0.0, 1.0));
    var color = textureSampleLevel(input_tex, linear_sampler, uv, 0.0);

    // Static snow inside tracking bar and head-switch zone
    if (dist_to_track < 0.03 || is_head_switch) {
        let snow = hash12(in.uv * globals.resolution + vec2<f32>(globals.time * 77.0, globals.time * 91.0));
        let snow_amount = select(0.35 * params.bar_intensity, 0.65, is_head_switch);
        color = vec4<f32>(mix(color.rgb, vec3<f32>(snow), snow_amount), color.a);
    }

    return color;
}
