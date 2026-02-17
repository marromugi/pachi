// ============================================================
// Eye SDF Shader
// Renders two eyes (left mirrored from right) using cubic
// Bezier outline for eye shape + SDF circles for iris/pupil.
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

    // Bezier outline: open state (128 bytes)
    // 4 segments × 2 vec4f. Each vec4f packs 2 vec2f control points.
    outline_open: array<vec4f, 8>,

    // Bezier outline: closed state (128 bytes)
    outline_closed: array<vec4f, 8>,
}

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
}

// ============================================================
// Constants
// ============================================================

const SUBDIV: u32 = 32u;

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

fn sd_circle(p: vec2f, r: f32) -> f32 {
    return length(p) - r;
}

// ============================================================
// Cubic Bezier evaluation
// ============================================================

fn cubic_bezier(t: f32, p0: vec2f, p1: vec2f, p2: vec2f, p3: vec2f) -> vec2f {
    let omt = 1.0 - t;
    let omt2 = omt * omt;
    let t2 = t * t;
    return omt2 * omt * p0 + 3.0 * omt2 * t * p1 + 3.0 * omt * t2 * p2 + t2 * t * p3;
}

// ============================================================
// Point-to-segment distance squared + winding number contribution
// Returns vec2f(distance_squared, winding)
// ============================================================

fn point_segment_test(p: vec2f, a: vec2f, b: vec2f) -> vec2f {
    let e = b - a;
    let w = p - a;
    let t = clamp(dot(w, e) / dot(e, e), 0.0, 1.0);
    let d = w - e * t;
    let d2 = dot(d, d);

    // Winding number contribution
    var winding = 0.0;
    if a.y <= p.y {
        if b.y > p.y {
            if e.x * w.y - e.y * w.x > 0.0 {
                winding = 1.0;
            }
        }
    } else {
        if b.y <= p.y {
            if e.x * w.y - e.y * w.x < 0.0 {
                winding = -1.0;
            }
        }
    }
    return vec2f(d2, winding);
}

// ============================================================
// Evaluate eye outline: returns signed distance
// (negative = inside, positive = outside)
// ============================================================

fn eval_outline(p: vec2f, close_t: f32) -> f32 {
    var min_d2 = 1e10;
    var winding = 0.0;

    for (var seg = 0u; seg < 4u; seg++) {
        let idx = seg * 2u;

        // Interpolate between open and closed control points
        let cp0 = mix(u.outline_open[idx], u.outline_closed[idx], close_t);
        let cp1 = mix(u.outline_open[idx + 1u], u.outline_closed[idx + 1u], close_t);

        let P0 = cp0.xy;
        let P1 = cp0.zw;
        let P2 = cp1.xy;
        let P3 = cp1.zw;

        var prev = P0;
        for (var i = 1u; i <= SUBDIV; i++) {
            let t = f32(i) / f32(SUBDIV);
            let curr = cubic_bezier(t, P0, P1, P2, P3);
            let result = point_segment_test(p, prev, curr);
            min_d2 = min(min_d2, result.x);
            winding += result.y;
            prev = curr;
        }
    }

    let dist = sqrt(min_d2);
    // winding != 0 means inside
    let sign_val = select(1.0, -1.0, winding != 0.0);
    return dist * sign_val;
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

    // --- Outline (replaces sclera ellipse + eyelid clipping) ---
    let d_outline = eval_outline(local_p, u.eyelid_close);
    let aa = fwidth(d_outline) * 0.5;
    let outline_mask = 1.0 - smoothstep(-aa, aa, d_outline);

    if outline_mask < 0.001 {
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

    return vec4f(eye_color, outline_mask);
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
