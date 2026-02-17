use egui;

use crate::outline::EyeShape;
use crate::EyeUniforms;

pub fn eye_control_panel(ctx: &egui::Context, uniforms: &mut EyeUniforms, eye_shape: &mut EyeShape) {
    egui::SidePanel::right("eye_controls")
        .default_width(280.0)
        .show(ctx, |ui| {
            ui.heading("Eye Controls");
            ui.separator();

            ui.add(
                egui::Slider::new(&mut uniforms.eyelid_close, 0.0..=1.0).text("Eyelid Close"),
            );

            let mut show = uniforms.show_iris_pupil > 0.5;
            if ui.checkbox(&mut show, "Show Iris & Pupil").changed() {
                uniforms.show_iris_pupil = if show { 1.0 } else { 0.0 };
            }

            ui.separator();

            egui::CollapsingHeader::new("Eye Shape")
                .default_open(true)
                .show(ui, |ui| {
                    eye_shape_editor(ui, eye_shape);
                    if ui.button("Reset Circle").clicked() {
                        eye_shape.open = crate::outline::BezierOutline::circle(0.30);
                    }
                });

            ui.separator();

            egui::CollapsingHeader::new("Appearance")
                .default_open(false)
                .show(ui, |ui| {
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
                    ui.horizontal(|ui| {
                        ui.label("Iris Inner");
                        color_edit_rgb(ui, &mut uniforms.iris_color_inner);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Iris Outer");
                        color_edit_rgb(ui, &mut uniforms.iris_color_outer);
                    });
                    ui.add(
                        egui::Slider::new(&mut uniforms.iris_noise_scale, 1.0..=20.0)
                            .text("Iris Noise"),
                    );
                });

            ui.separator();

            if ui.button("Reset All").clicked() {
                let aspect = uniforms.aspect_ratio;
                let time = uniforms.time;
                *uniforms = EyeUniforms::default();
                uniforms.aspect_ratio = aspect;
                uniforms.time = time;
                *eye_shape = EyeShape::default();
            }
        });
}

// ============================================================
// Interactive 2D Bezier curve editor
// ============================================================

// Drag target encoding: 0-3 = anchor[i], 4-7 = handle_in[i-4], 8-11 = handle_out[i-8]
const DRAG_NONE: i32 = -1;

fn eye_shape_editor(ui: &mut egui::Ui, eye_shape: &mut EyeShape) {
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
    let drag_id = response.id.with("drag");
    let mut drag_idx: i32 = ui.memory(|m| m.data.get_temp(drag_id)).unwrap_or(DRAG_NONE);

    // Find hovered point (for visual feedback)
    let hover_threshold = 12.0f32;
    let mut hovered_idx: i32 = DRAG_NONE;
    if drag_idx == DRAG_NONE {
        if let Some(pos) = response.hover_pos() {
            let mut best_dist = hover_threshold;
            for i in 0..4 {
                let a = &eye_shape.open.anchors[i];
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
    let anchors = &eye_shape.open.anchors;
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
                eye_shape.open.anchors[i].position = p;
                eye_shape.open.auto_adjust_handle_at(i);
            } else if drag_idx < 8 {
                // Dragging handle_in
                let i = (drag_idx - 4) as usize;
                let anchor = eye_shape.open.anchors[i].position;
                eye_shape.open.anchors[i].handle_in = [p[0] - anchor[0], p[1] - anchor[1]];
                eye_shape.open.anchors[i].enforce_collinear_from_in();
            } else {
                // Dragging handle_out
                let i = (drag_idx - 8) as usize;
                let anchor = eye_shape.open.anchors[i].position;
                eye_shape.open.anchors[i].handle_out = [p[0] - anchor[0], p[1] - anchor[1]];
                eye_shape.open.anchors[i].enforce_collinear_from_out();
            }
        }
    }

    if response.drag_stopped() {
        drag_idx = DRAG_NONE;
    }

    ui.memory_mut(|m| m.data.insert_temp(drag_id, drag_idx));
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
