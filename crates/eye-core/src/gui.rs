use egui;

use crate::outline::{BezierAnchor, BezierOutline, EyelashShape, EyeShape, EyebrowShape};
use crate::EyeUniforms;

// ============================================================
// Per-eye data types
// ============================================================

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Side {
    Left,
    Right,
}

/// Per-section link state: whether left/right eyes share the same parameters.
#[derive(Clone, Debug)]
pub struct SectionLink {
    pub linked: bool,
    /// Which eye is being edited when unlinked.
    pub active: Side,
}

impl Default for SectionLink {
    fn default() -> Self {
        Self {
            linked: true,
            active: Side::Left,
        }
    }
}

/// All parameters for one eye.
#[derive(Clone, Debug)]
pub struct EyeSideState {
    pub uniforms: EyeUniforms,
    pub eye_shape: EyeShape,
    pub eyebrow_shape: EyebrowShape,
    pub eyelash_shape: EyelashShape,
}

impl Default for EyeSideState {
    fn default() -> Self {
        Self {
            uniforms: EyeUniforms::default(),
            eye_shape: EyeShape::default(),
            eyebrow_shape: EyebrowShape::default(),
            eyelash_shape: EyelashShape::default(),
        }
    }
}

// ============================================================
// Section sync helpers
// ============================================================

fn sync_shape(from: &EyeSideState, to: &mut EyeSideState) {
    to.uniforms.eyelid_close = from.uniforms.eyelid_close;
    to.eye_shape = from.eye_shape.clone();
}

fn sync_iris(from: &EyeSideState, to: &mut EyeSideState) {
    to.uniforms.iris_color = from.uniforms.iris_color;
    to.uniforms.iris_radius = from.uniforms.iris_radius;
    to.uniforms.iris_follow = from.uniforms.iris_follow;
    to.uniforms.look_x = from.uniforms.look_x;
    to.uniforms.look_y = from.uniforms.look_y;
    to.uniforms.pupil_color = from.uniforms.pupil_color;
    to.uniforms.pupil_radius = from.uniforms.pupil_radius;
}

fn sync_eyebrow(from: &EyeSideState, to: &mut EyeSideState) {
    to.eyebrow_shape = from.eyebrow_shape.clone();
}

fn sync_eyelash(from: &EyeSideState, to: &mut EyeSideState) {
    to.eyelash_shape = from.eyelash_shape.clone();
}

/// Apply a section sync based on which side was active before re-linking.
fn apply_relink(
    from_side: Side,
    left: &mut EyeSideState,
    right: &mut EyeSideState,
    sync_fn: fn(&EyeSideState, &mut EyeSideState),
) {
    match from_side {
        Side::Left => sync_fn(&*left, right),
        Side::Right => sync_fn(&*right, left),
    }
}

// ============================================================
// Eye selector UI (Both / L / R)
// ============================================================

/// Renders the Both/L/R selector for a section.
/// Returns `Some(side)` if re-linked (transition from unlinked → linked),
/// indicating which side's values should be copied to the other.
fn section_eye_selector(ui: &mut egui::Ui, link: &mut SectionLink) -> Option<Side> {
    let mut relink_from = None;
    ui.horizontal(|ui| {
        if ui.selectable_label(link.linked, "Both").clicked() && !link.linked {
            relink_from = Some(link.active);
            link.linked = true;
        }
        if ui
            .selectable_label(!link.linked && link.active == Side::Left, "L")
            .clicked()
        {
            link.linked = false;
            link.active = Side::Left;
        }
        if ui
            .selectable_label(!link.linked && link.active == Side::Right, "R")
            .clicked()
        {
            link.linked = false;
            link.active = Side::Right;
        }
    });
    relink_from
}

// ============================================================
// Main control panel
// ============================================================

#[allow(clippy::too_many_arguments)]
pub fn eye_control_panel(
    ctx: &egui::Context,
    left: &mut EyeSideState,
    right: &mut EyeSideState,
    link_shape: &mut SectionLink,
    link_iris: &mut SectionLink,
    link_eyebrow: &mut SectionLink,
    link_eyelash: &mut SectionLink,
    auto_blink: &mut bool,
    follow_mouse: &mut bool,
    show_highlight: &mut bool,
    show_eyebrow: &mut bool,
    show_eyelash: &mut bool,
    focus_distance: &mut f32,
) {
    egui::SidePanel::right("eye_controls")
        .default_width(280.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Eye Controls");
                ui.separator();

                // --- Eyelid Close (linked to shape section) ---
                {
                    let editing_left = link_shape.linked || link_shape.active == Side::Left;
                    let eyelid = if editing_left {
                        &mut left.uniforms.eyelid_close
                    } else {
                        &mut right.uniforms.eyelid_close
                    };
                    let label = if link_shape.linked {
                        "Eyelid Close"
                    } else if link_shape.active == Side::Left {
                        "Eyelid Close [L]"
                    } else {
                        "Eyelid Close [R]"
                    };
                    ui.add_enabled(
                        !*auto_blink,
                        egui::Slider::new(eyelid, 0.0..=1.0).text(label),
                    );
                    if link_shape.linked {
                        right.uniforms.eyelid_close = left.uniforms.eyelid_close;
                    }
                }
                ui.checkbox(auto_blink, "Auto Blink");

                ui.separator();

                // --- 3D Perspective (always global, except look_x/look_y follow iris link) ---
                egui::CollapsingHeader::new("3D Perspective")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.checkbox(follow_mouse, "Follow Mouse");

                        // Look X/Y follow iris link state
                        {
                            let editing_left =
                                link_iris.linked || link_iris.active == Side::Left;
                            let look_uniforms = if editing_left {
                                &mut left.uniforms
                            } else {
                                &mut right.uniforms
                            };
                            let suffix = if link_iris.linked {
                                ""
                            } else if link_iris.active == Side::Left {
                                " [L]"
                            } else {
                                " [R]"
                            };
                            ui.add_enabled(
                                !*follow_mouse,
                                egui::Slider::new(&mut look_uniforms.look_x, -1.0..=1.0)
                                    .text(format!("Look X{suffix}")),
                            );
                            ui.add_enabled(
                                !*follow_mouse,
                                egui::Slider::new(&mut look_uniforms.look_y, -1.0..=1.0)
                                    .text(format!("Look Y{suffix}")),
                            );
                            if link_iris.linked {
                                right.uniforms.look_x = left.uniforms.look_x;
                                right.uniforms.look_y = left.uniforms.look_y;
                            }
                        }

                        // Global params (always edit left, sync to right)
                        ui.add(
                            egui::Slider::new(&mut left.uniforms.max_angle, 0.0..=1.5)
                                .text("Max Angle"),
                        );
                        right.uniforms.max_angle = left.uniforms.max_angle;

                        ui.add(
                            egui::Slider::new(&mut left.uniforms.eye_angle, 0.05..=1.2)
                                .text("Eye Angle"),
                        );
                        right.uniforms.eye_angle = left.uniforms.eye_angle;

                        ui.add(
                            egui::Slider::new(focus_distance, 0.5..=20.0)
                                .text("Focus Distance")
                                .logarithmic(true),
                        );
                    });

                ui.separator();

                // --- Iris / Pupil ---
                egui::CollapsingHeader::new("Iris")
                    .default_open(true)
                    .show(ui, |ui| {
                        if let Some(from) = section_eye_selector(ui, link_iris) {
                            apply_relink(from, left, right, sync_iris);
                        }

                        let editing_left = link_iris.linked || link_iris.active == Side::Left;
                        let u = if editing_left {
                            &mut left.uniforms
                        } else {
                            &mut right.uniforms
                        };

                        ui.horizontal(|ui| {
                            ui.label("Iris Color");
                            color_edit_rgb(ui, &mut u.iris_color);
                        });
                        ui.add(
                            egui::Slider::new(&mut u.iris_radius, 0.02..=0.25)
                                .text("Iris Radius"),
                        );
                        ui.add(
                            egui::Slider::new(&mut u.iris_follow, 0.0..=0.20)
                                .text("Iris Follow"),
                        );
                        ui.separator();
                        ui.label("Pupil");
                        ui.horizontal(|ui| {
                            ui.label("Pupil Color");
                            color_edit_rgb(ui, &mut u.pupil_color);
                        });
                        ui.add(
                            egui::Slider::new(&mut u.pupil_radius, 0.01..=0.20)
                                .text("Pupil Radius"),
                        );

                        // Sync linked fields
                        if link_iris.linked {
                            sync_iris(&*left, right);
                        }
                    });

                ui.separator();

                // --- Eye Shape ---
                egui::CollapsingHeader::new("Eye Shape")
                    .default_open(true)
                    .show(ui, |ui| {
                        if let Some(from) = section_eye_selector(ui, link_shape) {
                            apply_relink(from, left, right, sync_shape);
                        }

                        let editing_left = link_shape.linked || link_shape.active == Side::Left;
                        let eye_shape = if editing_left {
                            &mut left.eye_shape
                        } else {
                            &mut right.eye_shape
                        };
                        let side_suffix = if link_shape.linked {
                            ""
                        } else if link_shape.active == Side::Left {
                            "_left"
                        } else {
                            "_right"
                        };
                        let editor_id = format!("eye_shape{side_suffix}");
                        bezier_outline_editor(ui, &mut eye_shape.open, &editor_id);
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

                        // Sync linked fields
                        if link_shape.linked {
                            sync_shape(&*left, right);
                        }
                    });

                ui.separator();

                // --- Eyebrow ---
                egui::CollapsingHeader::new("Eyebrow")
                    .default_open(true)
                    .show(ui, |ui| {
                        if let Some(from) = section_eye_selector(ui, link_eyebrow) {
                            apply_relink(from, left, right, sync_eyebrow);
                        }

                        ui.checkbox(show_eyebrow, "Show Eyebrow");

                        let editing_left =
                            link_eyebrow.linked || link_eyebrow.active == Side::Left;
                        let eyebrow_shape = if editing_left {
                            &mut left.eyebrow_shape
                        } else {
                            &mut right.eyebrow_shape
                        };
                        let side_suffix = if link_eyebrow.linked {
                            ""
                        } else if link_eyebrow.active == Side::Left {
                            "_left"
                        } else {
                            "_right"
                        };

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
                        let editor_id = format!("eyebrow_shape{side_suffix}");
                        eyebrow_outline_editor(ui, &mut eyebrow_shape.outline, &editor_id);
                        // Tip thickness sliders (Left=0, Right=2)
                        for &(idx, label) in &[(0usize, "Tip L"), (2usize, "Tip R")] {
                            let a = &eyebrow_shape.outline.anchors[idx];
                            let len_in =
                                (a.handle_in[0].powi(2) + a.handle_in[1].powi(2)).sqrt();
                            let len_out =
                                (a.handle_out[0].powi(2) + a.handle_out[1].powi(2)).sqrt();
                            let mut thickness = len_in.max(len_out);
                            let old = thickness;
                            ui.add(
                                egui::Slider::new(&mut thickness, 0.001..=0.15).text(label),
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

                        // Sync linked fields
                        if link_eyebrow.linked {
                            sync_eyebrow(&*left, right);
                        }
                    });

                ui.separator();

                // --- Eyelash ---
                egui::CollapsingHeader::new("Eyelash")
                    .default_open(true)
                    .show(ui, |ui| {
                        if let Some(from) = section_eye_selector(ui, link_eyelash) {
                            apply_relink(from, left, right, sync_eyelash);
                        }

                        ui.checkbox(show_eyelash, "Show Eyelash");

                        let editing_left =
                            link_eyelash.linked || link_eyelash.active == Side::Left;
                        let eyelash_shape = if editing_left {
                            &mut left.eyelash_shape
                        } else {
                            &mut right.eyelash_shape
                        };

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

                        // Sync linked fields
                        if link_eyelash.linked {
                            sync_eyelash(&*left, right);
                        }
                    });

                ui.separator();

                // --- Appearance (always global) ---
                egui::CollapsingHeader::new("Appearance")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.checkbox(show_highlight, "Highlight");
                        ui.add(
                            egui::Slider::new(&mut left.uniforms.eye_separation, 0.2..=1.2)
                                .text("Eye Separation"),
                        );
                        right.uniforms.eye_separation = left.uniforms.eye_separation;

                        ui.horizontal(|ui| {
                            ui.label("BG Color");
                            color_edit_rgb(ui, &mut left.uniforms.bg_color);
                        });
                        right.uniforms.bg_color = left.uniforms.bg_color;

                        ui.horizontal(|ui| {
                            ui.label("Sclera Color");
                            color_edit_rgb(ui, &mut left.uniforms.sclera_color);
                        });
                        right.uniforms.sclera_color = left.uniforms.sclera_color;
                    });

                ui.separator();

                if ui.button("Reset All").clicked() {
                    let aspect = left.uniforms.aspect_ratio;
                    let time = left.uniforms.time;
                    *left = EyeSideState::default();
                    *right = EyeSideState::default();
                    left.uniforms.aspect_ratio = aspect;
                    left.uniforms.time = time;
                    right.uniforms.aspect_ratio = aspect;
                    right.uniforms.time = time;
                    *link_shape = SectionLink::default();
                    *link_iris = SectionLink::default();
                    *link_eyebrow = SectionLink::default();
                    *link_eyelash = SectionLink::default();
                }
            });
        });
}

// ============================================================
// Interactive 2D Bezier curve editor (generic)
// ============================================================

// Drag target encoding: 0-3 = anchor[i], 4-7 = handle_in[i-4], 8-11 = handle_out[i-8]
const DRAG_NONE: i32 = -1;

// ============================================================
// Blender-style modal editing state
// ============================================================

#[derive(Clone, Debug)]
struct BezierAnchorSnapshot {
    position: [f32; 2],
    handle_in: [f32; 2],
    handle_out: [f32; 2],
}

impl BezierAnchorSnapshot {
    fn from_anchor(a: &BezierAnchor) -> Self {
        Self {
            position: a.position,
            handle_in: a.handle_in,
            handle_out: a.handle_out,
        }
    }

    fn restore_to(&self, a: &mut BezierAnchor) {
        a.position = self.position;
        a.handle_in = self.handle_in;
        a.handle_out = self.handle_out;
    }
}

fn snapshot_all(anchors: &[BezierAnchor; 4]) -> [BezierAnchorSnapshot; 4] {
    core::array::from_fn(|i| BezierAnchorSnapshot::from_anchor(&anchors[i]))
}

fn restore_all(snaps: &[BezierAnchorSnapshot; 4], anchors: &mut [BezierAnchor; 4]) {
    for (s, a) in snaps.iter().zip(anchors.iter_mut()) {
        s.restore_to(a);
    }
}

#[derive(Clone, Debug)]
enum BezierEditMode {
    Idle,
    Grab {
        point_idx: i32,
        original_anchors: [BezierAnchorSnapshot; 4],
        /// Mouse position (screen coords) at the moment G was pressed.
        grab_origin: [f32; 2],
    },
    Scale {
        anchor_idx: usize,
        original_anchors: [BezierAnchorSnapshot; 4],
        anchor_screen_pos: [f32; 2],
        initial_mouse_dist: f32,
    },
    Rotate {
        anchor_idx: usize,
        original_anchors: [BezierAnchorSnapshot; 4],
        anchor_screen_pos: [f32; 2],
        initial_mouse_angle: f32,
    },
}

#[derive(Clone, Debug)]
struct BezierEditorState {
    drag_idx: i32,
    selected_idx: i32,
    mode: BezierEditMode,
    /// Skip the next click-to-select (set after modal confirm via click).
    skip_click_select: bool,
}

impl Default for BezierEditorState {
    fn default() -> Self {
        Self {
            drag_idx: DRAG_NONE,
            selected_idx: DRAG_NONE,
            mode: BezierEditMode::Idle,
            skip_click_select: false,
        }
    }
}

fn bezier_outline_editor(ui: &mut egui::Ui, outline: &mut BezierOutline, editor_id: &str) {
    let available_width = ui.available_width();
    let size = available_width.min(300.0);
    let (response, painter) = ui.allocate_painter(
        egui::vec2(size, size),
        egui::Sense::click_and_drag() | egui::Sense::FOCUSABLE,
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

    // --- Editor state ---
    let state_id = response.id.with(editor_id).with("editor_state");
    let mut es: BezierEditorState =
        ui.memory(|m| m.data.get_temp(state_id)).unwrap_or_default();

    // Find hovered point (for visual feedback)
    let hover_threshold = 12.0f32;
    let mut hovered_idx: i32 = DRAG_NONE;
    if es.drag_idx == DRAG_NONE && matches!(es.mode, BezierEditMode::Idle) {
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
    let select_ring_color = egui::Color32::from_rgb(100, 180, 255);

    for i in 0..4 {
        let a = &anchors[i];
        let hi = [a.position[0] + a.handle_in[0], a.position[1] + a.handle_in[1]];
        let ho = [a.position[0] + a.handle_out[0], a.position[1] + a.handle_out[1]];
        let hi_scr = to_screen(hi);
        let ho_scr = to_screen(ho);

        // Handle lines (draw behind points)
        painter.line_segment([hi_scr, ho_scr], egui::Stroke::new(1.0, handle_line_color));

        // Handle points
        let hi_active = hovered_idx == 4 + i as i32 || es.drag_idx == 4 + i as i32 || es.selected_idx == 4 + i as i32;
        let ho_active = hovered_idx == 8 + i as i32 || es.drag_idx == 8 + i as i32 || es.selected_idx == 8 + i as i32;
        painter.circle_filled(hi_scr, if hi_active { 5.0 } else { 3.5 }, if hi_active { handle_hover } else { handle_color });
        painter.circle_filled(ho_scr, if ho_active { 5.0 } else { 3.5 }, if ho_active { handle_hover } else { handle_color });

        // Selection rings for handles
        if es.selected_idx == 4 + i as i32 {
            painter.circle_stroke(hi_scr, 7.0, egui::Stroke::new(1.5, select_ring_color));
        }
        if es.selected_idx == 8 + i as i32 {
            painter.circle_stroke(ho_scr, 7.0, egui::Stroke::new(1.5, select_ring_color));
        }
    }

    // Draw anchor points (on top of everything)
    for i in 0..4 {
        let a_scr = to_screen(anchors[i].position);
        let active = hovered_idx == i as i32 || es.drag_idx == i as i32 || es.selected_idx == i as i32;
        painter.circle_filled(a_scr, if active { 7.0 } else { 5.0 }, if active { anchor_hover } else { anchor_color });

        // Selection ring for anchor
        if es.selected_idx == i as i32 {
            painter.circle_stroke(a_scr, 9.0, egui::Stroke::new(1.5, select_ring_color));
        }
    }

    // --- Mode indicator text ---
    match &es.mode {
        BezierEditMode::Grab { .. } => {
            painter.text(
                egui::pos2(rect.left() + 8.0, rect.top() + 8.0),
                egui::Align2::LEFT_TOP,
                "Grab (click=confirm, Esc=cancel)",
                egui::FontId::proportional(11.0),
                select_ring_color,
            );
        }
        BezierEditMode::Scale { .. } => {
            painter.text(
                egui::pos2(rect.left() + 8.0, rect.top() + 8.0),
                egui::Align2::LEFT_TOP,
                "Scale (click=confirm, Esc=cancel)",
                egui::FontId::proportional(11.0),
                select_ring_color,
            );
        }
        BezierEditMode::Rotate { .. } => {
            painter.text(
                egui::pos2(rect.left() + 8.0, rect.top() + 8.0),
                egui::Align2::LEFT_TOP,
                "Rotate (click=confirm, Esc=cancel)",
                egui::FontId::proportional(11.0),
                select_ring_color,
            );
        }
        BezierEditMode::Idle => {}
    }

    // --- Click-to-select (only in Idle mode) ---
    if matches!(es.mode, BezierEditMode::Idle) && response.clicked() {
        if es.skip_click_select {
            es.skip_click_select = false;
        } else if let Some(pos) = response.interact_pointer_pos() {
            let threshold = 15.0f32;
            let mut best_dist = threshold;
            let mut new_selected = DRAG_NONE;
            for i in 0..4 {
                let a = &outline.anchors[i];
                let d = pos.distance(to_screen(a.position));
                if d < best_dist {
                    best_dist = d;
                    new_selected = i as i32;
                }
                let hi = [a.position[0] + a.handle_in[0], a.position[1] + a.handle_in[1]];
                let d = pos.distance(to_screen(hi));
                if d < best_dist {
                    best_dist = d;
                    new_selected = 4 + i as i32;
                }
                let ho = [a.position[0] + a.handle_out[0], a.position[1] + a.handle_out[1]];
                let d = pos.distance(to_screen(ho));
                if d < best_dist {
                    best_dist = d;
                    new_selected = 8 + i as i32;
                }
            }
            es.selected_idx = new_selected;
            if es.selected_idx >= 0 {
                response.request_focus();
            }
        }
    }

    // --- Drag interaction (only in Idle mode) ---
    if matches!(es.mode, BezierEditMode::Idle) && response.drag_started() {
        if let Some(pos) = response.interact_pointer_pos() {
            let threshold = 15.0f32;
            let mut best_dist = threshold;
            es.drag_idx = DRAG_NONE;

            for i in 0..4 {
                let a = &outline.anchors[i];

                let d = pos.distance(to_screen(a.position));
                if d < best_dist {
                    best_dist = d;
                    es.drag_idx = i as i32;
                }

                let hi = [a.position[0] + a.handle_in[0], a.position[1] + a.handle_in[1]];
                let d = pos.distance(to_screen(hi));
                if d < best_dist {
                    best_dist = d;
                    es.drag_idx = 4 + i as i32;
                }

                let ho = [a.position[0] + a.handle_out[0], a.position[1] + a.handle_out[1]];
                let d = pos.distance(to_screen(ho));
                if d < best_dist {
                    best_dist = d;
                    es.drag_idx = 8 + i as i32;
                }
            }
        }
    }

    if matches!(es.mode, BezierEditMode::Idle) && response.dragged() && es.drag_idx >= 0 {
        if let Some(pos) = response.interact_pointer_pos() {
            let p = from_screen(pos);

            if es.drag_idx < 4 {
                let i = es.drag_idx as usize;
                outline.anchors[i].position = p;
            } else if es.drag_idx < 8 {
                let i = (es.drag_idx - 4) as usize;
                let anchor = outline.anchors[i].position;
                outline.anchors[i].handle_in = [p[0] - anchor[0], p[1] - anchor[1]];
                outline.anchors[i].enforce_collinear_from_in();
            } else {
                let i = (es.drag_idx - 8) as usize;
                let anchor = outline.anchors[i].position;
                outline.anchors[i].handle_out = [p[0] - anchor[0], p[1] - anchor[1]];
                outline.anchors[i].enforce_collinear_from_out();
            }
        }
    }

    if matches!(es.mode, BezierEditMode::Idle) && response.drag_stopped() {
        es.drag_idx = DRAG_NONE;
    }

    // --- Modal editing (G = Grab, S = Scale) ---
    let has_focus = response.has_focus();
    match es.mode.clone() {
        BezierEditMode::Idle => {
            if has_focus && es.selected_idx >= 0 {
                if ui.input(|i| i.key_pressed(egui::Key::G)) {
                    let mouse_pos = ui.input(|i| i.pointer.hover_pos())
                        .unwrap_or(egui::pos2(center.x, center.y));
                    es.mode = BezierEditMode::Grab {
                        point_idx: es.selected_idx,
                        original_anchors: snapshot_all(&outline.anchors),
                        grab_origin: [mouse_pos.x, mouse_pos.y],
                    };
                    ui.ctx().request_repaint();
                } else if ui.input(|i| i.key_pressed(egui::Key::S)) && es.selected_idx < 4 {
                    let anchor_scr = to_screen(outline.anchors[es.selected_idx as usize].position);
                    let mouse_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(anchor_scr);
                    let initial_dist = anchor_scr.distance(mouse_pos).max(1.0);
                    es.mode = BezierEditMode::Scale {
                        anchor_idx: es.selected_idx as usize,
                        original_anchors: snapshot_all(&outline.anchors),
                        anchor_screen_pos: [anchor_scr.x, anchor_scr.y],
                        initial_mouse_dist: initial_dist,
                    };
                    ui.ctx().request_repaint();
                } else if ui.input(|i| i.key_pressed(egui::Key::R)) {
                    let ai = if es.selected_idx < 4 {
                        es.selected_idx as usize
                    } else if es.selected_idx < 8 {
                        (es.selected_idx - 4) as usize
                    } else {
                        (es.selected_idx - 8) as usize
                    };
                    let anchor_scr = to_screen(outline.anchors[ai].position);
                    let mouse_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(anchor_scr);
                    let initial_angle = (mouse_pos.y - anchor_scr.y).atan2(mouse_pos.x - anchor_scr.x);
                    es.mode = BezierEditMode::Rotate {
                        anchor_idx: ai,
                        original_anchors: snapshot_all(&outline.anchors),
                        anchor_screen_pos: [anchor_scr.x, anchor_scr.y],
                        initial_mouse_angle: initial_angle,
                    };
                    ui.ctx().request_repaint();
                } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    es.selected_idx = DRAG_NONE;
                    response.surrender_focus();
                }
            }
        }
        BezierEditMode::Grab { point_idx, original_anchors, grab_origin } => {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                // Relative movement: delta from grab origin, applied to original position
                let delta = from_screen(mouse_pos);
                let origin = from_screen(egui::pos2(grab_origin[0], grab_origin[1]));
                let dx = delta[0] - origin[0];
                let dy = delta[1] - origin[1];

                if point_idx < 4 {
                    let i = point_idx as usize;
                    let orig = &original_anchors[i];
                    outline.anchors[i].position = [orig.position[0] + dx, orig.position[1] + dy];
                } else if point_idx < 8 {
                    let i = (point_idx - 4) as usize;
                    let orig = &original_anchors[i];
                    let new_hi = [orig.handle_in[0] + dx, orig.handle_in[1] + dy];
                    outline.anchors[i].handle_in = new_hi;
                    outline.anchors[i].enforce_collinear_from_in();
                } else {
                    let i = (point_idx - 8) as usize;
                    let orig = &original_anchors[i];
                    let new_ho = [orig.handle_out[0] + dx, orig.handle_out[1] + dy];
                    outline.anchors[i].handle_out = new_ho;
                    outline.anchors[i].enforce_collinear_from_out();
                }
            }

            if ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) {
                es.mode = BezierEditMode::Idle;
                es.skip_click_select = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                restore_all(&original_anchors, &mut outline.anchors);
                es.mode = BezierEditMode::Idle;
            }
            ui.ctx().request_repaint();
        }
        BezierEditMode::Scale { anchor_idx, original_anchors, anchor_screen_pos, initial_mouse_dist } => {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let anchor_scr = egui::pos2(anchor_screen_pos[0], anchor_screen_pos[1]);
                let current_dist = anchor_scr.distance(mouse_pos).max(1.0);
                let scale_factor = current_dist / initial_mouse_dist;

                let orig = &original_anchors[anchor_idx];
                outline.anchors[anchor_idx].handle_in = [
                    orig.handle_in[0] * scale_factor,
                    orig.handle_in[1] * scale_factor,
                ];
                outline.anchors[anchor_idx].handle_out = [
                    orig.handle_out[0] * scale_factor,
                    orig.handle_out[1] * scale_factor,
                ];
            }

            if ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) {
                es.mode = BezierEditMode::Idle;
                es.skip_click_select = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                restore_all(&original_anchors, &mut outline.anchors);
                es.mode = BezierEditMode::Idle;
            }
            ui.ctx().request_repaint();
        }
        BezierEditMode::Rotate { anchor_idx, original_anchors, anchor_screen_pos, initial_mouse_angle } => {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let anchor_scr = egui::pos2(anchor_screen_pos[0], anchor_screen_pos[1]);
                let current_angle = (mouse_pos.y - anchor_scr.y).atan2(mouse_pos.x - anchor_scr.x);
                let delta_angle = -(current_angle - initial_mouse_angle);
                let cos_a = delta_angle.cos();
                let sin_a = delta_angle.sin();

                let orig = &original_anchors[anchor_idx];
                outline.anchors[anchor_idx].handle_in = [
                    orig.handle_in[0] * cos_a - orig.handle_in[1] * sin_a,
                    orig.handle_in[0] * sin_a + orig.handle_in[1] * cos_a,
                ];
                outline.anchors[anchor_idx].handle_out = [
                    orig.handle_out[0] * cos_a - orig.handle_out[1] * sin_a,
                    orig.handle_out[0] * sin_a + orig.handle_out[1] * cos_a,
                ];
            }

            if ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) {
                es.mode = BezierEditMode::Idle;
                es.skip_click_select = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                restore_all(&original_anchors, &mut outline.anchors);
                es.mode = BezierEditMode::Idle;
            }
            ui.ctx().request_repaint();
        }
    }

    ui.memory_mut(|m| m.data.insert_temp(state_id, es));
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
        egui::Sense::click_and_drag() | egui::Sense::FOCUSABLE,
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

    // --- Editor state ---
    let state_id = response.id.with(editor_id).with("editor_state");
    let mut es: BezierEditorState =
        ui.memory(|m| m.data.get_temp(state_id)).unwrap_or_default();

    // Find hovered element (for visual feedback)
    let hover_threshold = 12.0f32;
    let mut hovered_idx: i32 = DRAG_NONE;
    if es.drag_idx == DRAG_NONE && matches!(es.mode, BezierEditMode::Idle) {
        if let Some(pos) = response.hover_pos() {
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
    let select_ring_color = egui::Color32::from_rgb(100, 180, 255);

    for i in 0..4 {
        let a = &anchors[i];
        let hi = extend_handle(a.position, a.handle_in);
        let ho = extend_handle(a.position, a.handle_out);
        let hi_scr = to_screen(hi);
        let ho_scr = to_screen(ho);

        painter.line_segment([hi_scr, ho_scr], egui::Stroke::new(1.0, handle_line_color));

        let hi_active = hovered_idx == 4 + i as i32 || es.drag_idx == 4 + i as i32 || es.selected_idx == 4 + i as i32;
        let ho_active = hovered_idx == 8 + i as i32 || es.drag_idx == 8 + i as i32 || es.selected_idx == 8 + i as i32;
        painter.circle_filled(hi_scr, if hi_active { 5.0 } else { 3.5 }, if hi_active { handle_hover } else { handle_color });
        painter.circle_filled(ho_scr, if ho_active { 5.0 } else { 3.5 }, if ho_active { handle_hover } else { handle_color });

        if es.selected_idx == 4 + i as i32 {
            painter.circle_stroke(hi_scr, 7.0, egui::Stroke::new(1.5, select_ring_color));
        }
        if es.selected_idx == 8 + i as i32 {
            painter.circle_stroke(ho_scr, 7.0, egui::Stroke::new(1.5, select_ring_color));
        }
    }

    for i in 0..4 {
        let a_scr = to_screen(anchors[i].position);
        let active = hovered_idx == i as i32 || es.drag_idx == i as i32 || es.selected_idx == i as i32;
        painter.circle_filled(a_scr, if active { 7.0 } else { 5.0 }, if active { anchor_hover } else { anchor_color });

        if es.selected_idx == i as i32 {
            painter.circle_stroke(a_scr, 9.0, egui::Stroke::new(1.5, select_ring_color));
        }
    }

    // --- Mode indicator text ---
    match &es.mode {
        BezierEditMode::Grab { .. } => {
            painter.text(
                egui::pos2(rect.left() + 8.0, rect.top() + 8.0),
                egui::Align2::LEFT_TOP,
                "Grab (click=confirm, Esc=cancel)",
                egui::FontId::proportional(11.0),
                select_ring_color,
            );
        }
        BezierEditMode::Scale { .. } => {
            painter.text(
                egui::pos2(rect.left() + 8.0, rect.top() + 8.0),
                egui::Align2::LEFT_TOP,
                "Scale (click=confirm, Esc=cancel)",
                egui::FontId::proportional(11.0),
                select_ring_color,
            );
        }
        BezierEditMode::Rotate { .. } => {
            painter.text(
                egui::pos2(rect.left() + 8.0, rect.top() + 8.0),
                egui::Align2::LEFT_TOP,
                "Rotate (click=confirm, Esc=cancel)",
                egui::FontId::proportional(11.0),
                select_ring_color,
            );
        }
        BezierEditMode::Idle => {}
    }

    // --- Click-to-select (only in Idle mode) ---
    if matches!(es.mode, BezierEditMode::Idle) && response.clicked() {
        if es.skip_click_select {
            es.skip_click_select = false;
        } else if let Some(pos) = response.interact_pointer_pos() {
            let threshold = 15.0f32;
            let mut best_dist = threshold;
            let mut new_selected = DRAG_NONE;
            for i in 0..4 {
                let a = &outline.anchors[i];
                let d = pos.distance(to_screen(a.position));
                if d < best_dist {
                    best_dist = d;
                    new_selected = i as i32;
                }
                let hi = extend_handle(a.position, a.handle_in);
                let d = pos.distance(to_screen(hi));
                if d < best_dist {
                    best_dist = d;
                    new_selected = 4 + i as i32;
                }
                let ho = extend_handle(a.position, a.handle_out);
                let d = pos.distance(to_screen(ho));
                if d < best_dist {
                    best_dist = d;
                    new_selected = 8 + i as i32;
                }
            }
            es.selected_idx = new_selected;
            if es.selected_idx >= 0 {
                response.request_focus();
            }
        }
    }

    // --- Drag interaction (only in Idle mode) ---
    if matches!(es.mode, BezierEditMode::Idle) && response.drag_started() {
        if let Some(pos) = response.interact_pointer_pos() {
            let threshold = 15.0f32;
            let mut best_dist = threshold;
            es.drag_idx = DRAG_NONE;

            for i in 0..4 {
                let a = &outline.anchors[i];

                let d = pos.distance(to_screen(a.position));
                if d < best_dist {
                    best_dist = d;
                    es.drag_idx = i as i32;
                }

                let hi = extend_handle(a.position, a.handle_in);
                let d = pos.distance(to_screen(hi));
                if d < best_dist {
                    best_dist = d;
                    es.drag_idx = 4 + i as i32;
                }

                let ho = extend_handle(a.position, a.handle_out);
                let d = pos.distance(to_screen(ho));
                if d < best_dist {
                    best_dist = d;
                    es.drag_idx = 8 + i as i32;
                }
            }
        }
    }

    if matches!(es.mode, BezierEditMode::Idle) && response.dragged() && es.drag_idx >= 0 {
        if let Some(pos) = response.interact_pointer_pos() {
            let p = from_screen(pos);

            if es.drag_idx < 4 {
                let i = es.drag_idx as usize;
                outline.anchors[i].position = p;
            } else if es.drag_idx < 8 {
                let i = (es.drag_idx - 4) as usize;
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
                let i = (es.drag_idx - 8) as usize;
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

    if matches!(es.mode, BezierEditMode::Idle) && response.drag_stopped() {
        es.drag_idx = DRAG_NONE;
    }

    // --- Modal editing (G = Grab, S = Scale) ---
    let has_focus = response.has_focus();
    match es.mode.clone() {
        BezierEditMode::Idle => {
            if has_focus && es.selected_idx >= 0 {
                if ui.input(|i| i.key_pressed(egui::Key::G)) {
                    let mouse_pos = ui.input(|i| i.pointer.hover_pos())
                        .unwrap_or(egui::pos2(center.x, center.y));
                    es.mode = BezierEditMode::Grab {
                        point_idx: es.selected_idx,
                        original_anchors: snapshot_all(&outline.anchors),
                        grab_origin: [mouse_pos.x, mouse_pos.y],
                    };
                    ui.ctx().request_repaint();
                } else if ui.input(|i| i.key_pressed(egui::Key::S)) && es.selected_idx < 4 {
                    let anchor_scr = to_screen(outline.anchors[es.selected_idx as usize].position);
                    let mouse_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(anchor_scr);
                    let initial_dist = anchor_scr.distance(mouse_pos).max(1.0);
                    es.mode = BezierEditMode::Scale {
                        anchor_idx: es.selected_idx as usize,
                        original_anchors: snapshot_all(&outline.anchors),
                        anchor_screen_pos: [anchor_scr.x, anchor_scr.y],
                        initial_mouse_dist: initial_dist,
                    };
                    ui.ctx().request_repaint();
                } else if ui.input(|i| i.key_pressed(egui::Key::R)) {
                    let ai = if es.selected_idx < 4 {
                        es.selected_idx as usize
                    } else if es.selected_idx < 8 {
                        (es.selected_idx - 4) as usize
                    } else {
                        (es.selected_idx - 8) as usize
                    };
                    let anchor_scr = to_screen(outline.anchors[ai].position);
                    let mouse_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(anchor_scr);
                    let initial_angle = (mouse_pos.y - anchor_scr.y).atan2(mouse_pos.x - anchor_scr.x);
                    es.mode = BezierEditMode::Rotate {
                        anchor_idx: ai,
                        original_anchors: snapshot_all(&outline.anchors),
                        anchor_screen_pos: [anchor_scr.x, anchor_scr.y],
                        initial_mouse_angle: initial_angle,
                    };
                    ui.ctx().request_repaint();
                } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    es.selected_idx = DRAG_NONE;
                    response.surrender_focus();
                }
            }
        }
        BezierEditMode::Grab { point_idx, original_anchors, grab_origin } => {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                // Relative movement: delta from grab origin, applied to original position
                let delta = from_screen(mouse_pos);
                let origin = from_screen(egui::pos2(grab_origin[0], grab_origin[1]));
                let dx = delta[0] - origin[0];
                let dy = delta[1] - origin[1];

                if point_idx < 4 {
                    let i = point_idx as usize;
                    let orig = &original_anchors[i];
                    outline.anchors[i].position = [orig.position[0] + dx, orig.position[1] + dy];
                } else if point_idx < 8 {
                    let i = (point_idx - 4) as usize;
                    let orig = &original_anchors[i];
                    let new_hi = [orig.handle_in[0] + dx, orig.handle_in[1] + dy];
                    // Tips (Left=0, Right=2): preserve handle length, change angle only
                    if i == 0 || i == 2 {
                        let old_len = (orig.handle_in[0].powi(2) + orig.handle_in[1].powi(2)).sqrt();
                        let new_len = (new_hi[0].powi(2) + new_hi[1].powi(2)).sqrt().max(1e-6);
                        let s = old_len / new_len;
                        outline.anchors[i].handle_in = [new_hi[0] * s, new_hi[1] * s];
                    } else {
                        outline.anchors[i].handle_in = new_hi;
                    }
                    outline.anchors[i].enforce_collinear_from_in();
                } else {
                    let i = (point_idx - 8) as usize;
                    let orig = &original_anchors[i];
                    let new_ho = [orig.handle_out[0] + dx, orig.handle_out[1] + dy];
                    // Tips (Left=0, Right=2): preserve handle length, change angle only
                    if i == 0 || i == 2 {
                        let old_len = (orig.handle_out[0].powi(2) + orig.handle_out[1].powi(2)).sqrt();
                        let new_len = (new_ho[0].powi(2) + new_ho[1].powi(2)).sqrt().max(1e-6);
                        let s = old_len / new_len;
                        outline.anchors[i].handle_out = [new_ho[0] * s, new_ho[1] * s];
                    } else {
                        outline.anchors[i].handle_out = new_ho;
                    }
                    outline.anchors[i].enforce_collinear_from_out();
                }
            }

            if ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) {
                es.mode = BezierEditMode::Idle;
                es.skip_click_select = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                restore_all(&original_anchors, &mut outline.anchors);
                es.mode = BezierEditMode::Idle;
            }
            ui.ctx().request_repaint();
        }
        BezierEditMode::Scale { anchor_idx, original_anchors, anchor_screen_pos, initial_mouse_dist } => {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let anchor_scr = egui::pos2(anchor_screen_pos[0], anchor_screen_pos[1]);
                let current_dist = anchor_scr.distance(mouse_pos).max(1.0);
                let scale_factor = current_dist / initial_mouse_dist;

                let orig = &original_anchors[anchor_idx];
                outline.anchors[anchor_idx].handle_in = [
                    orig.handle_in[0] * scale_factor,
                    orig.handle_in[1] * scale_factor,
                ];
                outline.anchors[anchor_idx].handle_out = [
                    orig.handle_out[0] * scale_factor,
                    orig.handle_out[1] * scale_factor,
                ];
            }

            if ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) {
                es.mode = BezierEditMode::Idle;
                es.skip_click_select = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                restore_all(&original_anchors, &mut outline.anchors);
                es.mode = BezierEditMode::Idle;
            }
            ui.ctx().request_repaint();
        }
        BezierEditMode::Rotate { anchor_idx, original_anchors, anchor_screen_pos, initial_mouse_angle } => {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let anchor_scr = egui::pos2(anchor_screen_pos[0], anchor_screen_pos[1]);
                let current_angle = (mouse_pos.y - anchor_scr.y).atan2(mouse_pos.x - anchor_scr.x);
                let delta_angle = -(current_angle - initial_mouse_angle);
                let cos_a = delta_angle.cos();
                let sin_a = delta_angle.sin();

                let orig = &original_anchors[anchor_idx];
                outline.anchors[anchor_idx].handle_in = [
                    orig.handle_in[0] * cos_a - orig.handle_in[1] * sin_a,
                    orig.handle_in[0] * sin_a + orig.handle_in[1] * cos_a,
                ];
                outline.anchors[anchor_idx].handle_out = [
                    orig.handle_out[0] * cos_a - orig.handle_out[1] * sin_a,
                    orig.handle_out[0] * sin_a + orig.handle_out[1] * cos_a,
                ];
            }

            if ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) {
                es.mode = BezierEditMode::Idle;
                es.skip_click_select = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                restore_all(&original_anchors, &mut outline.anchors);
                es.mode = BezierEditMode::Idle;
            }
            ui.ctx().request_repaint();
        }
    }

    ui.memory_mut(|m| m.data.insert_temp(state_id, es));
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
        s.push_str("            BezierAnchor {\n");
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
