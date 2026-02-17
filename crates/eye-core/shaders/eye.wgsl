// ============================================================
// Eye SDF Shader
// Renders two eyes (left mirrored from right) using cubic
// Bezier outline for eye shape.
// ============================================================

struct Uniforms {
    // Sclera (16 bytes)
    sclera_color: vec3f,
    squash_stretch: f32,

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
    look_x: f32,

    // Perspective (16 bytes)
    look_y: f32,
    max_angle: f32,
    eye_angle: f32,
    _pad_perspective: f32,

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
// Render a single eye at local coordinates.
// `mirror` is 1.0 for left eye, -1.0 for right eye.
// Returns (color, alpha).
// ============================================================

fn render_eye(p: vec2f, mirror: f32, h_scale: f32, v_scale: f32) -> vec4f {
    let foreshortened = vec2f(p.x / h_scale, p.y / v_scale);
    let local_p = vec2f(foreshortened.x * mirror, foreshortened.y);

    // --- Squash & Stretch (volume-preserving scale) ---
    let ss_scale = 1.0 + u.squash_stretch;
    let sq_p = vec2f(local_p.x / ss_scale, local_p.y * ss_scale);

    // --- Outline (replaces sclera ellipse + eyelid clipping) ---
    let d_outline = eval_outline(sq_p, u.eyelid_close);
    let aa = fwidth(d_outline) * 0.5;
    let outline_mask = 1.0 - smoothstep(-aa, aa, d_outline);

    if outline_mask < 0.001 {
        return vec4f(0.0, 0.0, 0.0, 0.0);
    }

    // --- Compose eye content ---
    var eye_color = u.sclera_color;

    // --- Highlight (additive, over everything) ---
    let look_shift = vec2f(u.look_x * 0.03, u.look_y * 0.03);
    let hl_p = sq_p - u.highlight_offset - look_shift;
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

    // --- Sphere projection model ---
    // Eyes are decals on a virtual sphere. Rotation causes foreshortening.
    let yaw   = u.look_x * u.max_angle;
    let pitch = u.look_y * u.max_angle * u.aspect_ratio * 1.3;

    // Angular half-separation of eyes on the sphere
    let half_sep = clamp(u.eye_angle, 0.01, 1.5);

    // Sphere radius: at yaw=0, eye_separation/2 = R * sin(half_sep)
    let R = u.eye_separation * 0.5 / sin(half_sep);

    // Per-eye horizontal angle from viewer center
    let left_h  = -half_sep + yaw;
    let right_h =  half_sep + yaw;

    // Screen positions (sphere projection, vertical dampened for "rotate in place" feel)
    let left_x  = R * sin(left_h);
    let right_x = R * sin(right_h);
    let y_off   = R * sin(pitch) * 0.3;

    // Foreshortening: cos(angle) compresses the eye shape
    let left_h_scale  = max(cos(left_h),  0.01);
    let right_h_scale = max(cos(right_h), 0.01);
    let v_scale       = max(cos(pitch),   0.01);

    let left_center  = vec2f(left_x,  y_off);
    let right_center = vec2f(right_x, y_off);

    // --- Render order: farther eye first, closer eye on top ---
    if left_h_scale <= right_h_scale {
        let left_p = p - left_center;
        let left = render_eye(left_p, 1.0, left_h_scale, v_scale);
        color = mix(color, left.xyz, left.w);

        let right_p = p - right_center;
        let right = render_eye(right_p, -1.0, right_h_scale, v_scale);
        color = mix(color, right.xyz, right.w);
    } else {
        let right_p = p - right_center;
        let right = render_eye(right_p, -1.0, right_h_scale, v_scale);
        color = mix(color, right.xyz, right.w);

        let left_p = p - left_center;
        let left = render_eye(left_p, 1.0, left_h_scale, v_scale);
        color = mix(color, left.xyz, left.w);
    }

    return vec4f(color, 1.0);
}
