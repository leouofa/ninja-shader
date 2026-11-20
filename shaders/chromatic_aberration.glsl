// Chromatic Aberration (GLSL)
// Globals and Params are auto-injected by ninja-shader

void main() {
    vec2 dir = uv - vec2(0.5, 0.5);
    float dist = length(dir);
    vec2 offset = vec2(0.0);
    if (dist > 0.0001) {
        offset = (dir / dist) * dist * dist * params.amount;
    }

    vec4 col_r = texture(sampler2D(input_tex, linear_sampler), uv - offset);
    vec4 col_g = texture(sampler2D(input_tex, linear_sampler), uv);
    vec4 col_b = texture(sampler2D(input_tex, linear_sampler), uv + offset);

    out_color = vec4(col_r.r, col_g.g, col_b.b, col_g.a);
}
