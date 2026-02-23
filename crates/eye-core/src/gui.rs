use egui;

use crate::outline::{BezierOutline, EyelashShape, EyeShape, EyebrowShape};
use crate::EyeUniforms;

pub fn eye_control_panel(ctx: &egui::Context, uniforms: &mut EyeUniforms, eye_shape: &mut EyeShape, eyebrow_shape: &mut EyebrowShape, eyelash_shape: &mut EyelashShape, auto_blink: &mut bool, follow_mouse: &mut bool, show_highlight: &mut bool, show_eyebrow: &mut bool, show_eyelash: &mut bool, focus_distance: &mut f32) {
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
                    ui.add(
                        egui::Slider::new(focus_distance, 0.5..=20.0)
                            .text("Focus Distance")
                            .logarithmic(true),
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
                    ui.separator();
                    ui.label("Pupil");
                    ui.horizontal(|ui| {
                        ui.label("Pupil Color");
                        color_edit_rgb(ui, &mut uniforms.pupil_color);
                    });
                    ui.add(
                        egui::Slider::new(&mut uniforms.pupil_radius, 0.01..=0.20)
                            .text("Pupil Radius"),
                    );
                });

            ui.separator();

            egui::CollapsingHeader::new("Eye Shape")
                .default_open(true)
                .show(ui, |ui| {
                    bezier_outline_editor(ui, &mut eye_shape.open, "eye_shape");
                    let old_arch = eye_shape.close_arch;
                    ui.add(
                        egui::Slider::new(&mut eye_shape.close_arch, -0.06..=0.06)
                            .text("Close Arch"),
                    );
                    if (eye_shape.close_arch - old_arch).abs() > 1e-6 {
                        eye_shape.update_closed();
                    }
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
                    // Tip thickness sliders (Left=0, Right=2)
                    for &(idx, label) in &[(0usize, "Tip L"), (2usize, "Tip R")] {
                        let a = &eyebrow_shape.outline.anchors[idx];
                        let len_in = (a.handle_in[0].powi(2) + a.handle_in[1].powi(2)).sqrt();
                        let len_out = (a.handle_out[0].powi(2) + a.handle_out[1].powi(2)).sqrt();
                        let mut thickness = len_in.max(len_out);
                        let old = thickness;
                        ui.add(
                            egui::Slider::new(&mut thickness, 0.001..=0.15)
                                .text(label),
                        );
                        if (thickness - old).abs() > 1e-6 {
                            let s = thickness / old.max(0.001);
                            eyebrow_shape.outline.anchors[idx].handle_in[0] *= s;
                            eyebrow_shape.outline.anchors[idx].handle_in[1] *= s;
                            eyebrow_shape.outline.anchors[idx].handle_out[0] *= s;
                            eyebrow_shape.outline.anchors[idx].handle_out[1] *= s;
                        }
                    }
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

            egui::CollapsingHeader::new("Eyelash")
                .default_open(true)
                .show(ui, |ui| {
                    ui.checkbox(show_eyelash, "Show Eyelash");
                    ui.horizontal(|ui| {
                        ui.label("Color");
                        color_edit_rgb(ui, &mut eyelash_shape.color);
                    });
                    ui.add(
                        egui::Slider::new(&mut eyelash_shape.thickness, 0.005..=0.06)
                            .text("Thickness"),
                    );
                    if ui.button("Reset Eyelash").clicked() {
                        *eyelash_shape = EyelashShape::default();
                    }
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
                *eyelash_shape = EyelashShape::default();
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
// 8-11 = handle_out[i-8]

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

    // Extend handle display position to ensure minimum distance from anchor (screen pixels).
    let min_handle_display_dist = 25.0f32;
    let extend_handle = |anchor_pos: [f32; 2], handle_offset: [f32; 2]| -> [f32; 2] {
        let abs = [anchor_pos[0] + handle_offset[0], anchor_pos[1] + handle_offset[1]];
        let scr_anchor = to_screen(anchor_pos);
        let scr_handle = to_screen(abs);
        let dist = scr_anchor.distance(scr_handle);
        if dist < min_handle_display_dist && dist > 1e-3 {
            let dir_x = scr_handle.x - scr_anchor.x;
            let dir_y = scr_handle.y - scr_anchor.y;
            let s = min_handle_display_dist / dist;
            let extended = egui::pos2(scr_anchor.x + dir_x * s, scr_anchor.y + dir_y * s);
            from_screen(extended)
        } else {
            abs
        }
    };

    // --- Drag state ---
    let drag_id = response.id.with(editor_id).with("drag");
    let mut drag_idx: i32 = ui.memory(|m| m.data.get_temp(drag_id)).unwrap_or(DRAG_NONE);

    // Find hovered element (for visual feedback)
    let hover_threshold = 12.0f32;
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
                let hi = extend_handle(a.position, a.handle_in);
                let d = pos.distance(to_screen(hi));
                if d < best_dist {
                    best_dist = d;
                    hovered_idx = 4 + i as i32;
                }
                let ho = extend_handle(a.position, a.handle_out);
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
        let hi = extend_handle(a.position, a.handle_in);
        let ho = extend_handle(a.position, a.handle_out);
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

                let hi = extend_handle(a.position, a.handle_in);
                let d = pos.distance(to_screen(hi));
                if d < best_dist {
                    best_dist = d;
                    drag_idx = 4 + i as i32;
                }

                let ho = extend_handle(a.position, a.handle_out);
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
                let i = drag_idx as usize;
                outline.anchors[i].position = p;
                // Tips (Left=0, Right=2): keep handles as-is for stable thickness
                if i == 1 || i == 3 {
                    outline.auto_adjust_handle_at(i);
                }
            } else if drag_idx < 8 {
                let i = (drag_idx - 4) as usize;
                let anchor = outline.anchors[i].position;
                let new_hi = [p[0] - anchor[0], p[1] - anchor[1]];
                // Tips (Left=0, Right=2): preserve handle length, change angle only
                if i == 0 || i == 2 {
                    let old_len = (outline.anchors[i].handle_in[0].powi(2) + outline.anchors[i].handle_in[1].powi(2)).sqrt();
                    let new_len = (new_hi[0].powi(2) + new_hi[1].powi(2)).sqrt().max(1e-6);
                    let s = old_len / new_len;
                    outline.anchors[i].handle_in = [new_hi[0] * s, new_hi[1] * s];
                } else {
                    outline.anchors[i].handle_in = new_hi;
                }
                outline.anchors[i].enforce_collinear_from_in();
            } else {
                let i = (drag_idx - 8) as usize;
                let anchor = outline.anchors[i].position;
                let new_ho = [p[0] - anchor[0], p[1] - anchor[1]];
                // Tips (Left=0, Right=2): preserve handle length, change angle only
                if i == 0 || i == 2 {
                    let old_len = (outline.anchors[i].handle_out[0].powi(2) + outline.anchors[i].handle_out[1].powi(2)).sqrt();
                    let new_len = (new_ho[0].powi(2) + new_ho[1].powi(2)).sqrt().max(1e-6);
                    let s = old_len / new_len;
                    outline.anchors[i].handle_out = [new_ho[0] * s, new_ho[1] * s];
                } else {
                    outline.anchors[i].handle_out = new_ho;
                }
                outline.anchors[i].enforce_collinear_from_out();
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
