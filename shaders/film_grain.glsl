// Film Grain Noise (GLSL)
// Globals and Params are auto-injected by ninja-shader

float hash(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

void main() {
    vec4 color = texture(sampler2D(input_tex, linear_sampler), uv);

    vec2 pixel_coord = floor(gl_FragCoord.xy / max(params.size, 1.0));
    float seed = float(globals.frame) * 17.13 + globals.time;
    float noise = hash(pixel_coord + vec2(seed, -seed)) * 2.0 - 1.0;

    vec3 noisy_rgb = color.rgb + vec3(noise * params.intensity);
    out_color = vec4(clamp(noisy_rgb, 0.0, 1.0), color.a);
}
