// ============================================================
// Eye SDF Shader
// Renders two eyes (left mirrored from right) using SDFs.
// ============================================================

struct Uniforms {
    // Sclera (16 bytes)
    sclera_color: vec3f,
    _pad0: f32,

    // Iris (48 bytes)
    iris_offset: vec2f,
    iris_radius: f32,
    iris_noise_scale: f32,
    iris_color_inner: vec3f,
    _pad1: f32,
    iris_color_outer: vec3f,
    _pad2: f32,

    // Pupil (16 bytes)
    pupil_color: vec3f,
    pupil_radius: f32,

    // Highlight (16 bytes)
    highlight_offset: vec2f,
    highlight_radius: f32,
    highlight_intensity: f32,

    // Global (32 bytes)
    bg_color: vec3f,
    eye_separation: f32,
    aspect_ratio: f32,
    time: f32,
    eyelid_close: f32,
    show_iris_pupil: f32,
}

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
}

// ============================================================
// Fixed eye shape constants
// ============================================================

const SCLERA_SIZE = vec2f(0.30, 0.30);

// ============================================================
// Eyelid Bezier interpolation
// ============================================================

struct EyelidPoints {
    upper_p0: vec2f,
    upper_p1: vec2f,
    upper_p2: vec2f,
    lower_p0: vec2f,
    lower_p1: vec2f,
    lower_p2: vec2f,
}

fn compute_eyelid(t: f32) -> EyelidPoints {
    // Open: upper corners above circle, lower corners below → no clipping → perfect circle
    // Closed: both converge to the lower meeting line (y ≈ -0.15)
    let upper_corner_y = mix(0.20, -0.15, t);
    let lower_corner_y = mix(-0.20, -0.15, t);

    var pts: EyelidPoints;
    pts.upper_p0 = vec2f(-0.35, upper_corner_y);
    pts.upper_p1 = vec2f(0.0, mix(0.40, -0.18, t));
    pts.upper_p2 = vec2f(0.35, upper_corner_y);
    pts.lower_p0 = vec2f(-0.35, lower_corner_y);
    pts.lower_p1 = vec2f(0.0, mix(-0.40, -0.18, t));
    pts.lower_p2 = vec2f(0.35, lower_corner_y);
    return pts;
}

// ============================================================
// Vertex shader: fullscreen triangle
// ============================================================

@vertex
fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
    let uv = vec2f(f32((id << 1u) & 2u), f32(id & 2u));
    var out: VertexOutput;
    out.clip_position = vec4f(uv * vec2f(2.0, -2.0) + vec2f(-1.0, 1.0), 0.0, 1.0);
    out.uv = uv;
    return out;
}

// ============================================================
// SDF primitives
// ============================================================

fn sd_ellipse(p: vec2f, ab: vec2f) -> f32 {
    let q = abs(p);
    let k = length(q / ab);
    if k < 0.0001 {
        return -min(ab.x, ab.y);
    }
    return (k - 1.0) * min(ab.x, ab.y);
}

fn sd_circle(p: vec2f, r: f32) -> f32 {
    return length(p) - r;
}

// ============================================================
// Quadratic bezier: evaluate Y at a given X.
// Assumes the curve is roughly horizontal (monotonic in X).
// ============================================================

fn bezier_y_at_x(x: f32, p0: vec2f, p1: vec2f, p2: vec2f) -> f32 {
    let dx = p2.x - p0.x;
    let t = clamp((x - p0.x) / (dx + sign(dx) * 0.0001), 0.0, 1.0);
    let omt = 1.0 - t;
    return omt * omt * p0.y + 2.0 * omt * t * p1.y + t * t * p2.y;
}

// ============================================================
// Hash-based noise for iris pattern
// ============================================================

fn hash21(p: vec2f) -> f32 {
    var p3 = fract(vec3f(p.x, p.y, p.x) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise2d(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let uu = f * f * (3.0 - 2.0 * f);

    return mix(
        mix(hash21(i + vec2f(0.0, 0.0)), hash21(i + vec2f(1.0, 0.0)), uu.x),
        mix(hash21(i + vec2f(0.0, 1.0)), hash21(i + vec2f(1.0, 1.0)), uu.x),
        uu.y
    );
}

fn fbm(p: vec2f) -> f32 {
    var value = 0.0;
    var amp = 0.5;
    var pos = p;
    for (var i = 0; i < 4; i++) {
        value += amp * noise2d(pos);
        amp *= 0.5;
        pos *= 2.0;
    }
    return value;
}

fn iris_pattern(p: vec2f, radius: f32, noise_scale: f32) -> f32 {
    let r = length(p) / radius;
    let angle = atan2(p.y, p.x);

    // Radial streaks
    let streaks = 0.5 + 0.5 * sin(angle * 16.0 + fbm(p * noise_scale) * 6.0);

    // Radial gradient (limbal ring = darker at edge)
    let rim = smoothstep(0.3, 0.95, r);

    return mix(1.0, streaks * 0.6 + 0.4, rim * 0.6);
}

// ============================================================
// Render a single eye at local coordinates.
// `mirror` is 1.0 for left eye, -1.0 for right eye.
// Returns (color, alpha).
// ============================================================

fn render_eye(p: vec2f, mirror: f32) -> vec4f {
    let local_p = vec2f(p.x * mirror, p.y);

    // --- Sclera (fixed oval shape) ---
    let d_sclera = sd_ellipse(local_p, SCLERA_SIZE);
    let aa_s = fwidth(d_sclera) * 0.5;
    let sclera_mask = 1.0 - smoothstep(-aa_s, aa_s, d_sclera);

    if sclera_mask < 0.001 {
        return vec4f(0.0, 0.0, 0.0, 0.0);
    }

    // --- Iris ---
    let iris_p = local_p - u.iris_offset;
    let d_iris = sd_circle(iris_p, u.iris_radius);
    let aa_i = fwidth(d_iris) * 0.5;
    let iris_mask = 1.0 - smoothstep(-aa_i, aa_i, d_iris);

    // Iris color with radial gradient + pattern
    let iris_r = clamp(length(iris_p) / u.iris_radius, 0.0, 1.0);
    let iris_base = mix(u.iris_color_inner, u.iris_color_outer, smoothstep(0.2, 0.95, iris_r));
    let pattern = iris_pattern(iris_p, u.iris_radius, u.iris_noise_scale);
    let iris_col = iris_base * pattern;

    // --- Pupil ---
    let d_pupil = sd_circle(iris_p, u.pupil_radius);
    let aa_p = fwidth(d_pupil) * 0.5;
    let pupil_mask = 1.0 - smoothstep(-aa_p, aa_p, d_pupil);

    // --- Compose eye content ---
    var eye_color = u.sclera_color;
    let ip_vis = u.show_iris_pupil;
    eye_color = mix(eye_color, iris_col, iris_mask * ip_vis);
    eye_color = mix(eye_color, u.pupil_color, pupil_mask * ip_vis);

    // --- Highlight (additive, over everything) ---
    let hl_p = iris_p - u.highlight_offset;
    let d_hl = sd_circle(hl_p, u.highlight_radius);
    let aa_h = fwidth(d_hl) * 0.5;
    let hl_mask = 1.0 - smoothstep(-aa_h, aa_h, d_hl);
    eye_color = eye_color + vec3f(u.highlight_intensity * hl_mask);

    // --- Eyelid clipping (computed from eyelid_close) ---
    let lid = compute_eyelid(u.eyelid_close);
    let upper_y = bezier_y_at_x(local_p.x, lid.upper_p0, lid.upper_p1, lid.upper_p2);
    let lower_y = bezier_y_at_x(local_p.x, lid.lower_p0, lid.lower_p1, lid.lower_p2);
    let lid_aa = fwidth(local_p.y) * 1.5;
    let lid_mask = smoothstep(-lid_aa, lid_aa, upper_y - local_p.y)
                 * smoothstep(-lid_aa, lid_aa, local_p.y - lower_y);

    // Final alpha
    let alpha = sclera_mask * lid_mask;

    return vec4f(eye_color, alpha);
}

// ============================================================
// Fragment shader
// ============================================================

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let p = vec2f(
        (in.uv.x * 2.0 - 1.0) * u.aspect_ratio,
        -(in.uv.y * 2.0 - 1.0)
    );

    var color = u.bg_color;

    // Left eye
    let left_center = vec2f(-u.eye_separation * 0.5, 0.0);
    let left_p = p - left_center;
    let left = render_eye(left_p, 1.0);
    color = mix(color, left.xyz, left.w);

    // Right eye (mirrored X)
    let right_center = vec2f(u.eye_separation * 0.5, 0.0);
    let right_p = p - right_center;
    let right = render_eye(right_p, -1.0);
    color = mix(color, right.xyz, right.w);

    return vec4f(color, 1.0);
}
