// Digital Glitch Shader
// Globals and Params are auto-injected by ninja-shader

fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = floor(globals.time * params.speed);
    let band_y = floor(in.uv.y * params.block_size);
    let noise = hash12(vec2<f32>(band_y, t));

    var uv = in.uv;
    var shift = 0.0;

    // Trigger glitch on slices based on intensity threshold
    if (noise < params.intensity) {
        let displacement = (hash12(vec2<f32>(band_y * 1.7, t * 2.3)) - 0.5) * 2.0;
        shift = displacement * params.intensity * 0.15;
        uv.x = fract(uv.x + shift);
    }

    let chroma = params.chroma_shift * params.intensity;
    let r = textureSampleLevel(input_tex, linear_sampler, vec2<f32>(uv.x + chroma, uv.y), 0.0).r;
    let g = textureSampleLevel(input_tex, linear_sampler, uv, 0.0).g;
    let b = textureSampleLevel(input_tex, linear_sampler, vec2<f32>(uv.x - chroma, uv.y), 0.0).b;
    let a = textureSampleLevel(input_tex, linear_sampler, uv, 0.0).a;

    return vec4<f32>(r, g, b, a);
}
