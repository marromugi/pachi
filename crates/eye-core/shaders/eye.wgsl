// ============================================================
// Eye SDF Shader
// Renders two eyes with independent per-eye parameters using
// cubic Bezier outline for eye shape.
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
    convergence: f32,

    // Iris (32 bytes)
    iris_color: vec3f,
    iris_radius: f32,
    iris_follow: f32,
    iris_offset_y: f32,
    _pad_iris_b: f32,
    _pad_iris_c: f32,

    // Bezier outline: open state (128 bytes)
    // 4 segments × 2 vec4f. Each vec4f packs 2 vec2f control points.
    outline_open: array<vec4f, 8>,

    // Bezier outline: closed state (128 bytes)
    outline_closed: array<vec4f, 8>,

    // Eyebrow (224 bytes)
    eyebrow_color: vec3f,
    eyebrow_base_y: f32,
    eyebrow_follow: f32,
    _pad_eyebrow_a: f32,
    _pad_eyebrow_b: f32,
    _pad_eyebrow_c: f32,
    eyebrow_outline: array<vec4f, 12>,

    // Eyelash (16 bytes) — stroke on upper eye outline
    eyelash_color: vec3f,
    eyelash_thickness: f32,

    // Pupil (16 bytes) — dark circle at center of iris
    pupil_color: vec3f,
    pupil_radius: f32,

    // Iris Bezier outline (128 bytes)
    iris_outline: array<vec4f, 8>,

    // Pupil Bezier outline (128 bytes)
    pupil_outline: array<vec4f, 8>,
}

struct EyePair {
    left: Uniforms,
    right: Uniforms,
}

@group(0) @binding(0)
var<uniform> pair: EyePair;

// Active eye parameters — set to pair.left or pair.right before rendering each eye.
var<private> u: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
}

// ============================================================
// Constants
// ============================================================

const SUBDIV: u32 = 16u;

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
// Evaluate eyebrow outline: returns signed distance
// The eyebrow is shifted vertically based on eyelid state.
// ============================================================

fn eval_eyebrow_outline(p: vec2f, close_t: f32) -> f32 {
    let y_offset = u.eyebrow_base_y - close_t * u.eyebrow_follow;
    let shifted_p = vec2f(p.x, p.y - y_offset);

    var min_d2 = 1e10;
    var winding = 0.0;

    for (var seg = 0u; seg < 6u; seg++) {
        let idx = seg * 2u;
        let cp0 = u.eyebrow_outline[idx];
        let cp1 = u.eyebrow_outline[idx + 1u];

        let P0 = cp0.xy;
        let P1 = cp0.zw;
        let P2 = cp1.xy;
        let P3 = cp1.zw;

        var prev = P0;
        for (var i = 1u; i <= SUBDIV; i++) {
            let t = f32(i) / f32(SUBDIV);
            let curr = cubic_bezier(t, P0, P1, P2, P3);
            let result = point_segment_test(shifted_p, prev, curr);
            min_d2 = min(min_d2, result.x);
            winding += result.y;
            prev = curr;
        }
    }

    let dist = sqrt(min_d2);
    let sign_val = select(1.0, -1.0, winding != 0.0);
    return dist * sign_val;
}

// ============================================================
// Evaluate iris outline: returns signed distance
// (negative = inside, positive = outside)
// ============================================================

fn eval_iris_outline(p: vec2f) -> f32 {
    var min_d2 = 1e10;
    var winding = 0.0;

    for (var seg = 0u; seg < 4u; seg++) {
        let idx = seg * 2u;
        let cp0 = u.iris_outline[idx];
        let cp1 = u.iris_outline[idx + 1u];

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
    let sign_val = select(1.0, -1.0, winding != 0.0);
    return dist * sign_val;
}

// ============================================================
// Evaluate pupil outline: returns signed distance
// (negative = inside, positive = outside)
// ============================================================

fn eval_pupil_outline(p: vec2f) -> f32 {
    var min_d2 = 1e10;
    var winding = 0.0;

    for (var seg = 0u; seg < 4u; seg++) {
        let idx = seg * 2u;
        let cp0 = u.pupil_outline[idx];
        let cp1 = u.pupil_outline[idx + 1u];

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
    let sign_val = select(1.0, -1.0, winding != 0.0);
    return dist * sign_val;
}

// ============================================================
// Render eyebrow at local coordinates.
// Same foreshortening/mirroring as the eye, but no squash_stretch.
// ============================================================

fn render_eyebrow(p: vec2f, mirror: f32, h_scale: f32, v_scale: f32, rest_h_scale: f32) -> vec4f {
    let foreshortened = vec2f(p.x / h_scale, p.y / v_scale);
    let local_p = vec2f(foreshortened.x * mirror, foreshortened.y);

    // Apply rest_h_scale to cancel rest-position foreshortening (WYSIWYG at yaw=0)
    let corrected_p = vec2f(local_p.x * rest_h_scale, local_p.y);
    let d_brow = eval_eyebrow_outline(corrected_p, u.eyelid_close);
    let aa = fwidth(d_brow) * 0.5;
    let brow_mask = 1.0 - smoothstep(-aa, aa, d_brow);

    if brow_mask < 0.001 {
        return vec4f(0.0, 0.0, 0.0, 0.0);
    }

    return vec4f(u.eyebrow_color, brow_mask);
}

// ============================================================
// Evaluate unsigned distance to the upper eye outline
// (segments 0: Left→Top, 1: Top→Right only).
// Returns vec2f(distance, t_along) where t_along is 0..1
// parametric position along the upper arc (0=Left, 0.5=Top, 1=Right).
// ============================================================

fn eval_upper_outline_dist(p: vec2f, close_t: f32) -> vec2f {
    var min_d2 = 1e10;
    var best_t = 0.5;
    let total_steps = f32(2u * SUBDIV);

    // Only upper 2 segments (Left→Top, Top→Right)
    for (var seg = 0u; seg < 2u; seg++) {
        let idx = seg * 2u;
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
            let e = curr - prev;
            let w = p - prev;
            let t_proj = clamp(dot(w, e) / dot(e, e), 0.0, 1.0);
            let d = w - e * t_proj;
            let d2 = dot(d, d);
            if d2 < min_d2 {
                min_d2 = d2;
                best_t = (f32(seg) * f32(SUBDIV) + f32(i - 1u) + t_proj) / total_steps;
            }
            prev = curr;
        }
    }

    return vec2f(sqrt(min_d2), best_t);
}

// ============================================================
// Render eyelash as a stroke on the upper eye outline.
// Follows the eye contour exactly, including during blinks.
// Tapers from full thickness at center to ~1px at tips.
// ============================================================

fn render_eyelash(p: vec2f, mirror: f32, h_scale: f32, v_scale: f32, rest_h_scale: f32) -> vec4f {
    let foreshortened = vec2f(p.x / h_scale, p.y / v_scale);
    let local_p = vec2f(foreshortened.x * mirror, foreshortened.y);

    // Apply same squash/stretch as the eye
    let ss_scale = 1.0 + u.squash_stretch;
    let sq_p = vec2f(local_p.x / ss_scale, local_p.y * ss_scale);

    // Apply rest_h_scale to match outline correction
    let corrected_p = vec2f(sq_p.x * rest_h_scale, sq_p.y);
    let result = eval_upper_outline_dist(corrected_p, u.eyelid_close);
    let dist = result.x;
    let t_along = result.y;  // 0=Left tip, 0.5=Top center, 1=Right tip

    // Taper: sin curve peaks at center, fades to ~0 at tips
    let taper = sin(t_along * 3.14159265);
    // Ensure minimum ~1px visible at tips
    let pixel_size = fwidth(corrected_p.x);
    let effective_thickness = max(u.eyelash_thickness * taper, pixel_size * 0.5);

    let aa = fwidth(dist) * 0.5;
    let lash_mask = 1.0 - smoothstep(effective_thickness - aa, effective_thickness + aa, dist);

    if lash_mask < 0.001 {
        return vec4f(0.0, 0.0, 0.0, 0.0);
    }

    return vec4f(u.eyelash_color, lash_mask);
}

// ============================================================
// Render a single eye at local coordinates.
// `mirror` is 1.0 for left eye, -1.0 for right eye.
// Returns (color, alpha).
// ============================================================

fn render_eye(p: vec2f, mirror: f32, h_scale: f32, v_scale: f32, rest_h_scale: f32) -> vec4f {
    let foreshortened = vec2f(p.x / h_scale, p.y / v_scale);
    let local_p = vec2f(foreshortened.x * mirror, foreshortened.y);

    // --- Squash & Stretch (volume-preserving scale) ---
    let ss_scale = 1.0 + u.squash_stretch;
    let sq_p = vec2f(local_p.x / ss_scale, local_p.y * ss_scale);

    // --- Outline (replaces sclera ellipse + eyelid clipping) ---
    // Apply rest_h_scale to cancel rest-position foreshortening (WYSIWYG at yaw=0)
    let outline_p = vec2f(sq_p.x * rest_h_scale, sq_p.y);
    let d_outline = eval_outline(outline_p, u.eyelid_close);
    let aa = fwidth(d_outline) * 0.5;
    let outline_mask = 1.0 - smoothstep(-aa, aa, d_outline);

    if outline_mask < 0.001 {
        return vec4f(0.0, 0.0, 0.0, 0.0);
    }

    // --- Compose eye content ---
    var eye_color = u.sclera_color;

    // --- Iris (follows gaze) ---
    let iris_offset = vec2f(
        mirror * u.look_x * u.iris_follow + u.convergence,
        u.look_y * u.iris_follow + u.iris_offset_y
    );
    let iris_p = sq_p - iris_offset;
    // Correct iris/pupil query coords: multiply x by rest_h_scale to cancel
    // the rest-position foreshortening, so editor shapes = screen shapes at yaw=0.
    let iris_p_shape = vec2f(iris_p.x * rest_h_scale, iris_p.y);
    let d_iris = eval_iris_outline(iris_p_shape);
    let aa_i = fwidth(d_iris) * 0.5;
    let iris_mask = 1.0 - smoothstep(-aa_i, aa_i, d_iris);
    eye_color = mix(eye_color, u.iris_color, iris_mask);

    // --- Pupil (center of iris) ---
    let d_pupil = eval_pupil_outline(iris_p_shape);
    let aa_p = fwidth(d_pupil) * 0.5;
    let pupil_mask = 1.0 - smoothstep(-aa_p, aa_p, d_pupil);
    eye_color = mix(eye_color, u.pupil_color, pupil_mask);

    // --- Highlight (additive, over everything) ---
    let look_shift = vec2f(u.look_x * 0.05, u.look_y * 0.05);
    let hl_p = outline_p - u.highlight_offset - look_shift;
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
    // Scene-level parameters from left (Rust side keeps global params in sync)
    let g = pair.left;

    let p = vec2f(
        (in.uv.x * 2.0 - 1.0) * g.aspect_ratio,
        -(in.uv.y * 2.0 - 1.0)
    );

    var color = g.bg_color;

    // --- Sphere projection model ---
    // Eyes are decals on a virtual sphere. Rotation causes foreshortening.
    let yaw   = g.look_x * g.max_angle;
    let pitch = g.look_y * g.max_angle * g.aspect_ratio * 0.65;

    // Angular half-separation of eyes on the sphere
    let half_sep = clamp(g.eye_angle, 0.01, 1.5);

    // Rest-position foreshortening factor for iris/pupil WYSIWYG correction.
    // At yaw=0, h_scale = cos(half_sep). Multiplying iris/pupil query coords
    // by this factor cancels the rest foreshortening so editor shapes = screen shapes.
    let rest_h_scale = cos(half_sep);

    // Sphere radius: at yaw=0, eye_separation/2 = R * sin(half_sep)
    let R = g.eye_separation * 0.5 / sin(half_sep);

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
    // Per eye: eyebrow first (behind), then eye on top.
    // Bounding box half-extents in local (pre-foreshortening) space.
    // Conservative: covers eye (~0.35) + eyebrow (base_y ~0.48 + height ~0.08)
    // + margin for squash_stretch and anti-aliasing.
    const BBOX_HX: f32 = 0.70;
    const BBOX_HY: f32 = 0.90;
    const BBOX_FADE: f32 = 0.10;

    if left_h_scale <= right_h_scale {
        u = pair.left;
        let left_p = p - left_center;
        let left_lx = abs(left_p.x) / left_h_scale;
        let left_ly = abs(left_p.y) / v_scale;
        if left_lx < BBOX_HX && left_ly < BBOX_HY {
            let fade = (1.0 - smoothstep(BBOX_HX - BBOX_FADE, BBOX_HX, left_lx))
                     * (1.0 - smoothstep(BBOX_HY - BBOX_FADE, BBOX_HY, left_ly));
            let left_brow = render_eyebrow(left_p, 1.0, left_h_scale, v_scale, rest_h_scale);
            color = mix(color, left_brow.xyz, left_brow.w * fade);
            let left = render_eye(left_p, 1.0, left_h_scale, v_scale, rest_h_scale);
            color = mix(color, left.xyz, left.w * fade);
            let left_lash = render_eyelash(left_p, 1.0, left_h_scale, v_scale, rest_h_scale);
            color = mix(color, left_lash.xyz, left_lash.w * fade);
        }

        u = pair.right;
        let right_p = p - right_center;
        let right_lx = abs(right_p.x) / right_h_scale;
        let right_ly = abs(right_p.y) / v_scale;
        if right_lx < BBOX_HX && right_ly < BBOX_HY {
            let fade = (1.0 - smoothstep(BBOX_HX - BBOX_FADE, BBOX_HX, right_lx))
                     * (1.0 - smoothstep(BBOX_HY - BBOX_FADE, BBOX_HY, right_ly));
            let right_brow = render_eyebrow(right_p, -1.0, right_h_scale, v_scale, rest_h_scale);
            color = mix(color, right_brow.xyz, right_brow.w * fade);
            let right = render_eye(right_p, -1.0, right_h_scale, v_scale, rest_h_scale);
            color = mix(color, right.xyz, right.w * fade);
            let right_lash = render_eyelash(right_p, -1.0, right_h_scale, v_scale, rest_h_scale);
            color = mix(color, right_lash.xyz, right_lash.w * fade);
        }
    } else {
        u = pair.right;
        let right_p = p - right_center;
        let right_lx = abs(right_p.x) / right_h_scale;
        let right_ly = abs(right_p.y) / v_scale;
        if right_lx < BBOX_HX && right_ly < BBOX_HY {
            let fade = (1.0 - smoothstep(BBOX_HX - BBOX_FADE, BBOX_HX, right_lx))
                     * (1.0 - smoothstep(BBOX_HY - BBOX_FADE, BBOX_HY, right_ly));
            let right_brow = render_eyebrow(right_p, -1.0, right_h_scale, v_scale, rest_h_scale);
            color = mix(color, right_brow.xyz, right_brow.w * fade);
            let right = render_eye(right_p, -1.0, right_h_scale, v_scale, rest_h_scale);
            color = mix(color, right.xyz, right.w * fade);
            let right_lash = render_eyelash(right_p, -1.0, right_h_scale, v_scale, rest_h_scale);
            color = mix(color, right_lash.xyz, right_lash.w * fade);
        }

        u = pair.left;
        let left_p = p - left_center;
        let left_lx = abs(left_p.x) / left_h_scale;
        let left_ly = abs(left_p.y) / v_scale;
        if left_lx < BBOX_HX && left_ly < BBOX_HY {
            let fade = (1.0 - smoothstep(BBOX_HX - BBOX_FADE, BBOX_HX, left_lx))
                     * (1.0 - smoothstep(BBOX_HY - BBOX_FADE, BBOX_HY, left_ly));
            let left_brow = render_eyebrow(left_p, 1.0, left_h_scale, v_scale, rest_h_scale);
            color = mix(color, left_brow.xyz, left_brow.w * fade);
            let left = render_eye(left_p, 1.0, left_h_scale, v_scale, rest_h_scale);
            color = mix(color, left.xyz, left.w * fade);
            let left_lash = render_eyelash(left_p, 1.0, left_h_scale, v_scale, rest_h_scale);
            color = mix(color, left_lash.xyz, left_lash.w * fade);
        }
    }

    return vec4f(color, 1.0);
}
