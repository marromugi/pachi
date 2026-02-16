struct Uniforms {
    left_center: vec2f,
    left_size: vec2f,
    right_center: vec2f,
    right_size: vec2f,
    eye_color: vec3f,
    _pad1: f32,
    bg_color: vec3f,
    aspect_ratio: f32,
}

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
}

@vertex
fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
    let uv = vec2f(f32((id << 1u) & 2u), f32(id & 2u));
    var out: VertexOutput;
    out.clip_position = vec4f(uv * vec2f(2.0, -2.0) + vec2f(-1.0, 1.0), 0.0, 1.0);
    out.uv = uv;
    return out;
}

fn sd_ellipse(p: vec2f, ab: vec2f) -> f32 {
    let q = abs(p);
    let k = length(q / ab);
    if k < 0.0001 {
        return -min(ab.x, ab.y);
    }
    return (k - 1.0) * min(ab.x, ab.y);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let p = vec2f(
        (in.uv.x * 2.0 - 1.0) * u.aspect_ratio,
        -(in.uv.y * 2.0 - 1.0)
    );

    var color = u.bg_color;

    // Left eye
    let dl = sd_ellipse(p - u.left_center, u.left_size);
    let aa_l = fwidth(dl) * 0.5;
    color = mix(color, u.eye_color, 1.0 - smoothstep(-aa_l, aa_l, dl));

    // Right eye
    let dr = sd_ellipse(p - u.right_center, u.right_size);
    let aa_r = fwidth(dr) * 0.5;
    color = mix(color, u.eye_color, 1.0 - smoothstep(-aa_r, aa_r, dr));

    return vec4f(color, 1.0);
}
