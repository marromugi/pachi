/// Cubic Bezier curve outline for eye shape morphing.
///
/// The eye outline is defined as a closed path of 4 cubic Bezier segments,
/// connecting 4 anchor points (Left, Top, Right, Bottom). Each anchor has
/// two handles (handle_in, handle_out) that are constrained to be collinear.

/// Kappa constant for cubic Bezier circle approximation.
/// A circle of radius r is approximated by 4 cubic Bezier segments
/// where each handle length = r * KAPPA.
const KAPPA: f32 = 0.552_284_749_8;

#[derive(Clone, Debug)]
pub struct BezierAnchor {
    /// Anchor point position (absolute coordinates).
    pub position: [f32; 2],
    /// Incoming handle offset (relative to anchor, points toward previous anchor).
    pub handle_in: [f32; 2],
    /// Outgoing handle offset (relative to anchor, points toward next anchor).
    pub handle_out: [f32; 2],
}

impl BezierAnchor {
    /// Enforce collinear constraint: keep handle_in opposite to handle_out
    /// while preserving handle_in's original length.
    pub fn enforce_collinear_from_out(&mut self) {
        let out_len = (self.handle_out[0].powi(2) + self.handle_out[1].powi(2)).sqrt();
        if out_len < 1e-8 {
            return;
        }
        let in_len = (self.handle_in[0].powi(2) + self.handle_in[1].powi(2)).sqrt();
        let dir = [-self.handle_out[0] / out_len, -self.handle_out[1] / out_len];
        self.handle_in = [dir[0] * in_len, dir[1] * in_len];
    }

    /// Enforce collinear constraint: keep handle_out opposite to handle_in
    /// while preserving handle_out's original length.
    pub fn enforce_collinear_from_in(&mut self) {
        let in_len = (self.handle_in[0].powi(2) + self.handle_in[1].powi(2)).sqrt();
        if in_len < 1e-8 {
            return;
        }
        let out_len = (self.handle_out[0].powi(2) + self.handle_out[1].powi(2)).sqrt();
        let dir = [-self.handle_in[0] / in_len, -self.handle_in[1] / in_len];
        self.handle_out = [dir[0] * out_len, dir[1] * out_len];
    }
}

#[derive(Clone, Debug)]
pub struct BezierOutline {
    /// 4 anchor points: [Left, Top, Right, Bottom] (counterclockwise).
    /// Segments: Left→Top, Top→Right, Right→Bottom, Bottom→Left.
    pub anchors: [BezierAnchor; 4],
}

impl BezierOutline {
    /// Create a circle approximation with the given radius.
    pub fn circle(radius: f32) -> Self {
        let h = radius * KAPPA;
        Self {
            anchors: [
                // Left (-r, 0): handle_in goes down, handle_out goes up
                BezierAnchor {
                    position: [-radius, 0.0],
                    handle_in: [0.0, -h],
                    handle_out: [0.0, h],
                },
                // Top (0, r): handle_in goes left, handle_out goes right
                BezierAnchor {
                    position: [0.0, radius],
                    handle_in: [-h, 0.0],
                    handle_out: [h, 0.0],
                },
                // Right (r, 0): handle_in goes up, handle_out goes down
                BezierAnchor {
                    position: [radius, 0.0],
                    handle_in: [0.0, h],
                    handle_out: [0.0, -h],
                },
                // Bottom (0, -r): handle_in goes right, handle_out goes left
                BezierAnchor {
                    position: [0.0, -radius],
                    handle_in: [h, 0.0],
                    handle_out: [-h, 0.0],
                },
            ],
        }
    }

    /// Create a closed-eye slit shape (nearly flat horizontal line).
    pub fn closed_slit(half_width: f32, y_pos: f32) -> Self {
        let tiny = 0.005;
        let hw = half_width * KAPPA;
        Self {
            anchors: [
                // Left corner
                BezierAnchor {
                    position: [-half_width, y_pos],
                    handle_in: [0.0, -tiny],
                    handle_out: [0.0, tiny],
                },
                // Top (barely above center)
                BezierAnchor {
                    position: [0.0, y_pos + tiny],
                    handle_in: [-hw, 0.0],
                    handle_out: [hw, 0.0],
                },
                // Right corner
                BezierAnchor {
                    position: [half_width, y_pos],
                    handle_in: [0.0, tiny],
                    handle_out: [0.0, -tiny],
                },
                // Bottom (barely below center)
                BezierAnchor {
                    position: [0.0, y_pos - tiny],
                    handle_in: [hw, 0.0],
                    handle_out: [-hw, 0.0],
                },
            ],
        }
    }

    /// Create a closed-eye slit with asymmetric eyelid movement.
    ///
    /// The upper lid (Top anchor) drops well below the Left/Right corners
    /// to form a reverse arch, while the lower lid (Bottom anchor) barely
    /// moves upward. The shader's linear `mix()` between open and closed
    /// states naturally produces: arch → flat → reverse arch on the upper lid.
    pub fn closed_slit_asymmetric(half_width: f32, y_slit: f32) -> Self {
        let tiny = 0.005;
        let reverse_h = 0.015; // how far upper lid dips below slit corners
        let hw = half_width * KAPPA;

        Self {
            anchors: [
                // Left corner — sits at slit level
                BezierAnchor {
                    position: [-half_width, y_slit],
                    handle_in: [0.0, -tiny],
                    handle_out: [0.0, tiny],
                },
                // Top (upper lid) — dips BELOW slit level for reverse arch
                BezierAnchor {
                    position: [0.0, y_slit - reverse_h],
                    handle_in: [-hw, 0.0],
                    handle_out: [hw, 0.0],
                },
                // Right corner — sits at slit level
                BezierAnchor {
                    position: [half_width, y_slit],
                    handle_in: [0.0, tiny],
                    handle_out: [0.0, -tiny],
                },
                // Bottom (lower lid) — just below Top to avoid crossing
                BezierAnchor {
                    position: [0.0, y_slit - reverse_h - tiny],
                    handle_in: [hw, 0.0],
                    handle_out: [-hw, 0.0],
                },
            ],
        }
    }

    /// Convert to a flat array of 8 × [f32; 4] for GPU uniform upload.
    ///
    /// Layout: For segment i (connecting anchor[i] to anchor[(i+1)%4]):
    ///   uniform[i*2]   = [P0.x, P0.y, P1.x, P1.y]  (anchor, anchor+handle_out)
    ///   uniform[i*2+1] = [P2.x, P2.y, P3.x, P3.y]  (next_anchor+handle_in, next_anchor)
    pub fn to_uniform_array(&self) -> [[f32; 4]; 8] {
        let mut result = [[0.0f32; 4]; 8];
        for seg in 0..4 {
            let next = (seg + 1) % 4;
            let a = &self.anchors[seg];
            let b = &self.anchors[next];

            // P0 = anchor position
            let p0 = a.position;
            // P1 = anchor + handle_out (outgoing control point)
            let p1 = [
                a.position[0] + a.handle_out[0],
                a.position[1] + a.handle_out[1],
            ];
            // P2 = next_anchor + handle_in (incoming control point)
            let p2 = [
                b.position[0] + b.handle_in[0],
                b.position[1] + b.handle_in[1],
            ];
            // P3 = next anchor position
            let p3 = b.position;

            result[seg * 2] = [p0[0], p0[1], p1[0], p1[1]];
            result[seg * 2 + 1] = [p2[0], p2[1], p3[0], p3[1]];
        }
        result
    }

    /// Auto-adjust handles for a single anchor based on its neighbors.
    /// Only modifies anchor[i]'s handles; other anchors are untouched.
    pub fn auto_adjust_handle_at(&mut self, i: usize) {
        let n = self.anchors.len();
        let prev = (i + n - 1) % n;
        let next = (i + 1) % n;

        let to_prev = [
            self.anchors[prev].position[0] - self.anchors[i].position[0],
            self.anchors[prev].position[1] - self.anchors[i].position[1],
        ];
        let to_next = [
            self.anchors[next].position[0] - self.anchors[i].position[0],
            self.anchors[next].position[1] - self.anchors[i].position[1],
        ];

        let len_prev = (to_prev[0].powi(2) + to_prev[1].powi(2)).sqrt();
        let len_next = (to_next[0].powi(2) + to_next[1].powi(2)).sqrt();

        if len_prev < 1e-8 || len_next < 1e-8 {
            return;
        }

        // Direction: bisect the angle between prev and next
        let dir = [
            to_next[0] / len_next - to_prev[0] / len_prev,
            to_next[1] / len_next - to_prev[1] / len_prev,
        ];
        let dir_len = (dir[0].powi(2) + dir[1].powi(2)).sqrt();

        if dir_len < 1e-8 {
            let perp = [-to_next[1] / len_next, to_next[0] / len_next];
            self.anchors[i].handle_out = [perp[0] * len_next * KAPPA, perp[1] * len_next * KAPPA];
            self.anchors[i].handle_in = [-perp[0] * len_prev * KAPPA, -perp[1] * len_prev * KAPPA];
        } else {
            let dir_norm = [dir[0] / dir_len, dir[1] / dir_len];
            let out_len = len_next * KAPPA;
            let in_len = len_prev * KAPPA;

            self.anchors[i].handle_out = [dir_norm[0] * out_len, dir_norm[1] * out_len];
            self.anchors[i].handle_in = [-dir_norm[0] * in_len, -dir_norm[1] * in_len];
        }
    }

    /// Auto-adjust handles for all anchors.
    pub fn auto_adjust_handles(&mut self) {
        for i in 0..self.anchors.len() {
            self.auto_adjust_handle_at(i);
        }
    }
}

/// Holds both open and closed eye outline shapes.
/// The shader interpolates between them using `eyelid_close`.
#[derive(Clone, Debug)]
pub struct EyeShape {
    pub open: BezierOutline,
    pub closed: BezierOutline,
}

impl Default for EyeShape {
    fn default() -> Self {
        Self {
            open: BezierOutline::circle(0.30),
            closed: BezierOutline::closed_slit_asymmetric(0.30, -0.20),
        }
    }
}
