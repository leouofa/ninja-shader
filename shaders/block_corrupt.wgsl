// Block Corrupt / Macroblock Artifacting
// Globals and Params are auto-injected by ninja-shader

fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = floor(globals.time * params.speed);
    let block_coord = floor(in.uv * params.grid_scale);
    let rnd = hash12(block_coord + vec2<f32>(t * 13.1, t * 7.7));

    var uv = in.uv;
    var crush = false;
    var invert = false;

    if (rnd < params.corruption_rate) {
        // Swap block UV with random neighboring block
        let offset_x = (hash12(block_coord + vec2<f32>(t, 1.0)) - 0.5) * 0.4;
        let offset_y = (hash12(block_coord + vec2<f32>(t, 2.0)) - 0.5) * 0.4;
        uv = fract(in.uv + vec2<f32>(offset_x, offset_y));

        if (rnd < params.corruption_rate * 0.5) {
            crush = true;
        }
        if (rnd < params.corruption_rate * 0.25) {
            invert = true;
        }
    }

    var color = textureSampleLevel(input_tex, linear_sampler, uv, 0.0);

    if (crush) {
        let levels = max(params.color_crush, 1.0);
        color = vec4<f32>(floor(color.rgb * levels) / levels, color.a);
    }

    if (invert) {
        color = vec4<f32>(vec3<f32>(1.0) - color.rgb, color.a);
    }

    return color;
}
