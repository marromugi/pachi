use egui;

use crate::outline::{BezierOutline, EyeShape, EyebrowShape};
use crate::EyeUniforms;

pub fn eye_control_panel(ctx: &egui::Context, uniforms: &mut EyeUniforms, eye_shape: &mut EyeShape, eyebrow_shape: &mut EyebrowShape, auto_blink: &mut bool, follow_mouse: &mut bool, show_highlight: &mut bool, show_eyebrow: &mut bool) {
    egui::SidePanel::right("eye_controls")
        .default_width(280.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Eye Controls");
            ui.separator();

            ui.add_enabled(
                !*auto_blink,
                egui::Slider::new(&mut uniforms.eyelid_close, 0.0..=1.0).text("Eyelid Close"),
            );
            ui.checkbox(auto_blink, "Auto Blink");

            ui.separator();

            egui::CollapsingHeader::new("3D Perspective")
                .default_open(true)
                .show(ui, |ui| {
                    ui.checkbox(follow_mouse, "Follow Mouse");
                    ui.add_enabled(
                        !*follow_mouse,
                        egui::Slider::new(&mut uniforms.look_x, -1.0..=1.0).text("Look X"),
                    );
                    ui.add_enabled(
                        !*follow_mouse,
                        egui::Slider::new(&mut uniforms.look_y, -1.0..=1.0).text("Look Y"),
                    );
                    ui.add(
                        egui::Slider::new(&mut uniforms.max_angle, 0.0..=1.5)
                            .text("Max Angle"),
                    );
                    ui.add(
                        egui::Slider::new(&mut uniforms.eye_angle, 0.05..=1.2)
                            .text("Eye Angle"),
                    );
                });

            ui.separator();

            egui::CollapsingHeader::new("Iris")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Iris Color");
                        color_edit_rgb(ui, &mut uniforms.iris_color);
                    });
                    ui.add(
                        egui::Slider::new(&mut uniforms.iris_radius, 0.02..=0.25)
                            .text("Iris Radius"),
                    );
                    ui.add(
                        egui::Slider::new(&mut uniforms.iris_follow, 0.0..=0.20)
                            .text("Iris Follow"),
                    );
                });

            ui.separator();

            egui::CollapsingHeader::new("Eye Shape")
                .default_open(true)
                .show(ui, |ui| {
                    bezier_outline_editor(ui, &mut eye_shape.open, "eye_shape");
                    if ui.button("Reset Ellipse").clicked() {
                        eye_shape.open = BezierOutline::ellipse(0.28, 0.35);
                    }
                });

            ui.separator();

            egui::CollapsingHeader::new("Eyebrow")
                .default_open(true)
                .show(ui, |ui| {
                    ui.checkbox(show_eyebrow, "Show Eyebrow");
                    ui.horizontal(|ui| {
                        ui.label("Color");
                        color_edit_rgb(ui, &mut eyebrow_shape.color);
                    });
                    ui.add(
                        egui::Slider::new(&mut eyebrow_shape.base_y, 0.30..=0.70)
                            .text("Base Y"),
                    );
                    ui.add(
                        egui::Slider::new(&mut eyebrow_shape.follow, 0.0..=0.40)
                            .text("Follow Rate"),
                    );
                    eyebrow_outline_editor(ui, &mut eyebrow_shape.outline, "eyebrow_shape");
                    ui.horizontal(|ui| {
                        if ui.button("Reset Eyebrow").clicked() {
                            *eyebrow_shape = EyebrowShape::default();
                        }
                        if ui.button("Copy").clicked() {
                            let s = format_eyebrow_shape(eyebrow_shape);
                            ui.ctx().copy_text(s);
                        }
                    });
                });

            ui.separator();

            egui::CollapsingHeader::new("Appearance")
                .default_open(false)
                .show(ui, |ui| {
                    ui.checkbox(show_highlight, "Highlight");
                    ui.add(
                        egui::Slider::new(&mut uniforms.eye_separation, 0.2..=1.2)
                            .text("Eye Separation"),
                    );
                    ui.horizontal(|ui| {
                        ui.label("BG Color");
                        color_edit_rgb(ui, &mut uniforms.bg_color);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Sclera Color");
                        color_edit_rgb(ui, &mut uniforms.sclera_color);
                    });
                });

            ui.separator();

            if ui.button("Reset All").clicked() {
                let aspect = uniforms.aspect_ratio;
                let time = uniforms.time;
                *uniforms = EyeUniforms::default();
                uniforms.aspect_ratio = aspect;
                uniforms.time = time;
                *eye_shape = EyeShape::default();
                *eyebrow_shape = EyebrowShape::default();
            }
            });
        });
}

// ============================================================
// Interactive 2D Bezier curve editor (generic)
// ============================================================

// Drag target encoding: 0-3 = anchor[i], 4-7 = handle_in[i-4], 8-11 = handle_out[i-8]
const DRAG_NONE: i32 = -1;

fn bezier_outline_editor(ui: &mut egui::Ui, outline: &mut BezierOutline, editor_id: &str) {
    let available_width = ui.available_width();
    let size = available_width.min(300.0);
    let (response, painter) = ui.allocate_painter(
        egui::vec2(size, size),
        egui::Sense::click_and_drag(),
    );
    let rect = response.rect;

    // Coordinate mapping: eye space [-0.5, 0.5] -> canvas pixels
    let scale = rect.width() * 0.85;
    let center = rect.center();

    let to_screen = |p: [f32; 2]| -> egui::Pos2 {
        egui::pos2(center.x + p[0] * scale, center.y - p[1] * scale)
    };
    let from_screen = |p: egui::Pos2| -> [f32; 2] {
        [
            (p.x - center.x) / scale,
            -(p.y - center.y) / scale,
        ]
    };

    // --- Drag state ---
    let drag_id = response.id.with(editor_id).with("drag");
    let mut drag_idx: i32 = ui.memory(|m| m.data.get_temp(drag_id)).unwrap_or(DRAG_NONE);

    // Find hovered point (for visual feedback)
    let hover_threshold = 12.0f32;
    let mut hovered_idx: i32 = DRAG_NONE;
    if drag_idx == DRAG_NONE {
        if let Some(pos) = response.hover_pos() {
            let mut best_dist = hover_threshold;
            for i in 0..4 {
                let a = &outline.anchors[i];
                let d = pos.distance(to_screen(a.position));
                if d < best_dist {
                    best_dist = d;
                    hovered_idx = i as i32;
                }
                let hi = [a.position[0] + a.handle_in[0], a.position[1] + a.handle_in[1]];
                let d = pos.distance(to_screen(hi));
                if d < best_dist {
                    best_dist = d;
                    hovered_idx = 4 + i as i32;
                }
                let ho = [a.position[0] + a.handle_out[0], a.position[1] + a.handle_out[1]];
                let d = pos.distance(to_screen(ho));
                if d < best_dist {
                    best_dist = d;
                    hovered_idx = 8 + i as i32;
                }
            }
        }
    }

    // --- Background ---
    painter.rect_filled(rect, 4.0, egui::Color32::from_gray(30));

    // Grid crosshair
    let grid_color = egui::Color32::from_gray(55);
    painter.line_segment(
        [egui::pos2(rect.left(), center.y), egui::pos2(rect.right(), center.y)],
        egui::Stroke::new(0.5, grid_color),
    );
    painter.line_segment(
        [egui::pos2(center.x, rect.top()), egui::pos2(center.x, rect.bottom())],
        egui::Stroke::new(0.5, grid_color),
    );

    // --- Draw Bezier curve segments ---
    let curve_color = egui::Color32::from_rgb(220, 220, 220);
    let anchors = &outline.anchors;
    for seg in 0..4 {
        let next = (seg + 1) % 4;
        let a = &anchors[seg];
        let b = &anchors[next];
        let p0 = a.position;
        let p1 = [p0[0] + a.handle_out[0], p0[1] + a.handle_out[1]];
        let p3 = b.position;
        let p2 = [p3[0] + b.handle_in[0], p3[1] + b.handle_in[1]];

        let subdiv = 24;
        let mut prev = to_screen(p0);
        for j in 1..=subdiv {
            let t = j as f32 / subdiv as f32;
            let omt = 1.0 - t;
            let x = omt * omt * omt * p0[0]
                + 3.0 * omt * omt * t * p1[0]
                + 3.0 * omt * t * t * p2[0]
                + t * t * t * p3[0];
            let y = omt * omt * omt * p0[1]
                + 3.0 * omt * omt * t * p1[1]
                + 3.0 * omt * t * t * p2[1]
                + t * t * t * p3[1];
            let curr = to_screen([x, y]);
            painter.line_segment([prev, curr], egui::Stroke::new(2.0, curve_color));
            prev = curr;
        }
    }

    // --- Draw handle lines and handle points ---
    let handle_line_color = egui::Color32::from_gray(100);
    let handle_color = egui::Color32::from_rgb(255, 160, 0);
    let handle_hover = egui::Color32::from_rgb(255, 220, 100);
    let anchor_color = egui::Color32::WHITE;
    let anchor_hover = egui::Color32::from_rgb(255, 255, 180);

    for i in 0..4 {
        let a = &anchors[i];
        let hi = [a.position[0] + a.handle_in[0], a.position[1] + a.handle_in[1]];
        let ho = [a.position[0] + a.handle_out[0], a.position[1] + a.handle_out[1]];
        let hi_scr = to_screen(hi);
        let ho_scr = to_screen(ho);

        // Handle lines (draw behind points)
        painter.line_segment([hi_scr, ho_scr], egui::Stroke::new(1.0, handle_line_color));

        // Handle points
        let hi_active = hovered_idx == 4 + i as i32 || drag_idx == 4 + i as i32;
        let ho_active = hovered_idx == 8 + i as i32 || drag_idx == 8 + i as i32;
        painter.circle_filled(hi_scr, if hi_active { 5.0 } else { 3.5 }, if hi_active { handle_hover } else { handle_color });
        painter.circle_filled(ho_scr, if ho_active { 5.0 } else { 3.5 }, if ho_active { handle_hover } else { handle_color });
    }

    // Draw anchor points (on top of everything)
    for i in 0..4 {
        let a_scr = to_screen(anchors[i].position);
        let active = hovered_idx == i as i32 || drag_idx == i as i32;
        painter.circle_filled(a_scr, if active { 7.0 } else { 5.0 }, if active { anchor_hover } else { anchor_color });
    }

    // --- Drag interaction ---
    if response.drag_started() {
        if let Some(pos) = response.interact_pointer_pos() {
            let threshold = 15.0f32;
            let mut best_dist = threshold;
            drag_idx = DRAG_NONE;

            for i in 0..4 {
                let a = &anchors[i];

                // Check anchor
                let d = pos.distance(to_screen(a.position));
                if d < best_dist {
                    best_dist = d;
                    drag_idx = i as i32;
                }

                // Check handle_in
                let hi = [a.position[0] + a.handle_in[0], a.position[1] + a.handle_in[1]];
                let d = pos.distance(to_screen(hi));
                if d < best_dist {
                    best_dist = d;
                    drag_idx = 4 + i as i32;
                }

                // Check handle_out
                let ho = [a.position[0] + a.handle_out[0], a.position[1] + a.handle_out[1]];
                let d = pos.distance(to_screen(ho));
                if d < best_dist {
                    best_dist = d;
                    drag_idx = 8 + i as i32;
                }
            }
        }
    }

    if response.dragged() && drag_idx >= 0 {
        if let Some(pos) = response.interact_pointer_pos() {
            let p = from_screen(pos);

            if drag_idx < 4 {
                // Dragging an anchor point — only re-adjust this anchor's handles
                let i = drag_idx as usize;
                outline.anchors[i].position = p;
                outline.auto_adjust_handle_at(i);
            } else if drag_idx < 8 {
                // Dragging handle_in
                let i = (drag_idx - 4) as usize;
                let anchor = outline.anchors[i].position;
                outline.anchors[i].handle_in = [p[0] - anchor[0], p[1] - anchor[1]];
                outline.anchors[i].enforce_collinear_from_in();
            } else {
                // Dragging handle_out
                let i = (drag_idx - 8) as usize;
                let anchor = outline.anchors[i].position;
                outline.anchors[i].handle_out = [p[0] - anchor[0], p[1] - anchor[1]];
                outline.anchors[i].enforce_collinear_from_out();
            }
        }
    }

    if response.drag_stopped() {
        drag_idx = DRAG_NONE;
    }

    ui.memory_mut(|m| m.data.insert_temp(drag_id, drag_idx));
}

// ============================================================
// Eyebrow Bezier editor with thickness rings
// ============================================================

// Drag target encoding: 0-3 = anchor[i], 4-7 = handle_in[i-4],
// 8-11 = handle_out[i-8], 12-15 = thickness_ring[i-12]

fn eyebrow_outline_editor(ui: &mut egui::Ui, outline: &mut BezierOutline, editor_id: &str) {
    let available_width = ui.available_width();
    let size = available_width.min(300.0);
    let (response, painter) = ui.allocate_painter(
        egui::vec2(size, size),
        egui::Sense::click_and_drag(),
    );
    let rect = response.rect;

    let scale = rect.width() * 0.85;
    let center = rect.center();

    let to_screen = |p: [f32; 2]| -> egui::Pos2 {
        egui::pos2(center.x + p[0] * scale, center.y - p[1] * scale)
    };
    let from_screen = |p: egui::Pos2| -> [f32; 2] {
        [
            (p.x - center.x) / scale,
            -(p.y - center.y) / scale,
        ]
    };

    // Compute ring radii (eye-space) per anchor
    let ring_radii: [f32; 4] = std::array::from_fn(|i| {
        let a = &outline.anchors[i];
        let len_in = (a.handle_in[0].powi(2) + a.handle_in[1].powi(2)).sqrt();
        let len_out = (a.handle_out[0].powi(2) + a.handle_out[1].powi(2)).sqrt();
        len_in.max(len_out)
    });

    const MIN_RING_DISPLAY_RADIUS: f32 = 12.0;

    // --- Drag state ---
    let drag_id = response.id.with(editor_id).with("drag");
    let mut drag_idx: i32 = ui.memory(|m| m.data.get_temp(drag_id)).unwrap_or(DRAG_NONE);

    // Find hovered element (for visual feedback)
    let hover_threshold = 12.0f32;
    let ring_edge_threshold = 8.0f32;
    let mut hovered_idx: i32 = DRAG_NONE;
    if drag_idx == DRAG_NONE {
        if let Some(pos) = response.hover_pos() {
            // First pass: anchors and handles (higher priority)
            let mut best_dist = hover_threshold;
            for i in 0..4 {
                let a = &outline.anchors[i];
                let d = pos.distance(to_screen(a.position));
                if d < best_dist {
                    best_dist = d;
                    hovered_idx = i as i32;
                }
                let hi = [a.position[0] + a.handle_in[0], a.position[1] + a.handle_in[1]];
                let d = pos.distance(to_screen(hi));
                if d < best_dist {
                    best_dist = d;
                    hovered_idx = 4 + i as i32;
                }
                let ho = [a.position[0] + a.handle_out[0], a.position[1] + a.handle_out[1]];
                let d = pos.distance(to_screen(ho));
                if d < best_dist {
                    best_dist = d;
                    hovered_idx = 8 + i as i32;
                }
            }
            // Second pass: thickness rings on tip anchors only (Left=0, Right=2)
            if hovered_idx == DRAG_NONE {
                let mut best_ring_dist = ring_edge_threshold;
                for i in [0, 2] {
                    let a_scr = to_screen(outline.anchors[i].position);
                    let display_r = (ring_radii[i] * scale).max(MIN_RING_DISPLAY_RADIUS);
                    let dist_to_center = pos.distance(a_scr);
                    let dist_to_edge = (dist_to_center - display_r).abs();
                    if dist_to_edge < best_ring_dist {
                        best_ring_dist = dist_to_edge;
                        hovered_idx = 12 + i as i32;
                    }
                }
            }
        }
    }

    // --- Background ---
    painter.rect_filled(rect, 4.0, egui::Color32::from_gray(30));

    let grid_color = egui::Color32::from_gray(55);
    painter.line_segment(
        [egui::pos2(rect.left(), center.y), egui::pos2(rect.right(), center.y)],
        egui::Stroke::new(0.5, grid_color),
    );
    painter.line_segment(
        [egui::pos2(center.x, rect.top()), egui::pos2(center.x, rect.bottom())],
        egui::Stroke::new(0.5, grid_color),
    );

    // --- Draw thickness rings on tip anchors only (Left=0, Right=2) ---
    let ring_color = egui::Color32::from_rgb(0, 200, 220);
    let ring_hover_color = egui::Color32::from_rgb(100, 255, 255);

    for i in [0, 2] {
        let a_scr = to_screen(outline.anchors[i].position);
        let display_r = (ring_radii[i] * scale).max(MIN_RING_DISPLAY_RADIUS);
        let active = hovered_idx == 12 + i as i32 || drag_idx == 12 + i as i32;
        let stroke_width = if active { 2.5 } else { 1.5 };
        let color = if active { ring_hover_color } else { ring_color };
        painter.circle_stroke(a_scr, display_r, egui::Stroke::new(stroke_width, color));
    }

    // --- Draw Bezier curve segments ---
    let curve_color = egui::Color32::from_rgb(220, 220, 220);
    let anchors = &outline.anchors;
    for seg in 0..4 {
        let next = (seg + 1) % 4;
        let a = &anchors[seg];
        let b = &anchors[next];
        let p0 = a.position;
        let p1 = [p0[0] + a.handle_out[0], p0[1] + a.handle_out[1]];
        let p3 = b.position;
        let p2 = [p3[0] + b.handle_in[0], p3[1] + b.handle_in[1]];

        let subdiv = 24;
        let mut prev = to_screen(p0);
        for j in 1..=subdiv {
            let t = j as f32 / subdiv as f32;
            let omt = 1.0 - t;
            let x = omt * omt * omt * p0[0]
                + 3.0 * omt * omt * t * p1[0]
                + 3.0 * omt * t * t * p2[0]
                + t * t * t * p3[0];
            let y = omt * omt * omt * p0[1]
                + 3.0 * omt * omt * t * p1[1]
                + 3.0 * omt * t * t * p2[1]
                + t * t * t * p3[1];
            let curr = to_screen([x, y]);
            painter.line_segment([prev, curr], egui::Stroke::new(2.0, curve_color));
            prev = curr;
        }
    }

    // --- Draw handle lines and handle points ---
    let handle_line_color = egui::Color32::from_gray(100);
    let handle_color = egui::Color32::from_rgb(255, 160, 0);
    let handle_hover = egui::Color32::from_rgb(255, 220, 100);
    let anchor_color = egui::Color32::WHITE;
    let anchor_hover = egui::Color32::from_rgb(255, 255, 180);

    for i in 0..4 {
        let a = &anchors[i];
        let hi = [a.position[0] + a.handle_in[0], a.position[1] + a.handle_in[1]];
        let ho = [a.position[0] + a.handle_out[0], a.position[1] + a.handle_out[1]];
        let hi_scr = to_screen(hi);
        let ho_scr = to_screen(ho);

        painter.line_segment([hi_scr, ho_scr], egui::Stroke::new(1.0, handle_line_color));

        let hi_active = hovered_idx == 4 + i as i32 || drag_idx == 4 + i as i32;
        let ho_active = hovered_idx == 8 + i as i32 || drag_idx == 8 + i as i32;
        painter.circle_filled(hi_scr, if hi_active { 5.0 } else { 3.5 }, if hi_active { handle_hover } else { handle_color });
        painter.circle_filled(ho_scr, if ho_active { 5.0 } else { 3.5 }, if ho_active { handle_hover } else { handle_color });
    }

    for i in 0..4 {
        let a_scr = to_screen(anchors[i].position);
        let active = hovered_idx == i as i32 || drag_idx == i as i32;
        painter.circle_filled(a_scr, if active { 7.0 } else { 5.0 }, if active { anchor_hover } else { anchor_color });
    }

    // --- Drag interaction ---
    if response.drag_started() {
        if let Some(pos) = response.interact_pointer_pos() {
            let threshold = 15.0f32;
            let mut best_dist = threshold;
            drag_idx = DRAG_NONE;

            // Check anchors and handles first (higher priority)
            for i in 0..4 {
                let a = &anchors[i];

                let d = pos.distance(to_screen(a.position));
                if d < best_dist {
                    best_dist = d;
                    drag_idx = i as i32;
                }

                let hi = [a.position[0] + a.handle_in[0], a.position[1] + a.handle_in[1]];
                let d = pos.distance(to_screen(hi));
                if d < best_dist {
                    best_dist = d;
                    drag_idx = 4 + i as i32;
                }

                let ho = [a.position[0] + a.handle_out[0], a.position[1] + a.handle_out[1]];
                let d = pos.distance(to_screen(ho));
                if d < best_dist {
                    best_dist = d;
                    drag_idx = 8 + i as i32;
                }
            }

            // Check thickness rings on tips only (lower priority)
            if drag_idx == DRAG_NONE {
                let mut best_ring_dist = ring_edge_threshold;
                for i in [0, 2] {
                    let a_scr = to_screen(outline.anchors[i].position);
                    let display_r = (ring_radii[i] * scale).max(MIN_RING_DISPLAY_RADIUS);
                    let dist_to_center = pos.distance(a_scr);
                    let dist_to_edge = (dist_to_center - display_r).abs();
                    if dist_to_edge < best_ring_dist {
                        best_ring_dist = dist_to_edge;
                        drag_idx = 12 + i as i32;
                    }
                }
            }
        }
    }

    if response.dragged() && drag_idx >= 0 {
        if let Some(pos) = response.interact_pointer_pos() {
            let p = from_screen(pos);

            if drag_idx < 4 {
                let i = drag_idx as usize;
                outline.anchors[i].position = p;
                // Tips (Left=0, Right=2): keep handles as-is for stable thickness
                if i == 1 || i == 3 {
                    outline.auto_adjust_handle_at(i);
                }
            } else if drag_idx < 8 {
                let i = (drag_idx - 4) as usize;
                let anchor = outline.anchors[i].position;
                outline.anchors[i].handle_in = [p[0] - anchor[0], p[1] - anchor[1]];
                outline.anchors[i].enforce_collinear_from_in();
            } else if drag_idx < 12 {
                let i = (drag_idx - 8) as usize;
                let anchor = outline.anchors[i].position;
                outline.anchors[i].handle_out = [p[0] - anchor[0], p[1] - anchor[1]];
                outline.anchors[i].enforce_collinear_from_out();
            } else {
                // Dragging thickness ring — scale both handles uniformly
                let i = (drag_idx - 12) as usize;
                let anchor = outline.anchors[i].position;
                let new_dist = ((p[0] - anchor[0]).powi(2) + (p[1] - anchor[1]).powi(2)).sqrt();

                let len_in = (outline.anchors[i].handle_in[0].powi(2) + outline.anchors[i].handle_in[1].powi(2)).sqrt();
                let len_out = (outline.anchors[i].handle_out[0].powi(2) + outline.anchors[i].handle_out[1].powi(2)).sqrt();
                let current_max = len_in.max(len_out).max(0.003);

                let s = new_dist / current_max;
                outline.anchors[i].handle_in[0] *= s;
                outline.anchors[i].handle_in[1] *= s;
                outline.anchors[i].handle_out[0] *= s;
                outline.anchors[i].handle_out[1] *= s;
            }
        }
    }

    if response.drag_stopped() {
        drag_idx = DRAG_NONE;
    }

    ui.memory_mut(|m| m.data.insert_temp(drag_id, drag_idx));
}

fn format_eyebrow_shape(shape: &EyebrowShape) -> String {
    let mut s = String::from("EyebrowShape {\n");
    s.push_str(&format!("    base_y: {:.4},\n", shape.base_y));
    s.push_str(&format!("    follow: {:.4},\n", shape.follow));
    s.push_str(&format!("    color: [{:.4}, {:.4}, {:.4}],\n", shape.color[0], shape.color[1], shape.color[2]));
    s.push_str("    outline: BezierOutline {\n        anchors: [\n");
    let labels = ["Left", "Top", "Right", "Bottom"];
    for (i, a) in shape.outline.anchors.iter().enumerate() {
        s.push_str(&format!("            // {}\n", labels[i]));
        s.push_str(&format!("            BezierAnchor {{\n"));
        s.push_str(&format!("                position: [{:.6}, {:.6}],\n", a.position[0], a.position[1]));
        s.push_str(&format!("                handle_in: [{:.6}, {:.6}],\n", a.handle_in[0], a.handle_in[1]));
        s.push_str(&format!("                handle_out: [{:.6}, {:.6}],\n", a.handle_out[0], a.handle_out[1]));
        s.push_str("            },\n");
    }
    s.push_str("        ],\n    },\n}");
    s
}

fn color_edit_rgb(ui: &mut egui::Ui, color: &mut [f32; 3]) {
    let mut rgba = egui::Color32::from_rgb(
        (color[0] * 255.0) as u8,
        (color[1] * 255.0) as u8,
        (color[2] * 255.0) as u8,
    );
    if ui.color_edit_button_srgba(&mut rgba).changed() {
        color[0] = rgba.r() as f32 / 255.0;
        color[1] = rgba.g() as f32 / 255.0;
        color[2] = rgba.b() as f32 / 255.0;
    }
}
