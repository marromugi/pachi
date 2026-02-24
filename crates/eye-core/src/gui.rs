use egui;

use crate::outline::{BezierAnchor, BezierOutline, EyelashShape, EyeShape, EyebrowGuide, EyebrowOutline, EyebrowShape, IrisShape, PupilShape};
use crate::EyeUniforms;

// ============================================================
// GUI action signaling
// ============================================================

#[derive(Debug, Default)]
pub struct GuiActions {
    pub export_requested: bool,
}

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
    pub iris_shape: IrisShape,
    pub pupil_shape: PupilShape,
}

impl Default for EyeSideState {
    fn default() -> Self {
        Self {
            uniforms: EyeUniforms::default(),
            eye_shape: EyeShape::default(),
            eyebrow_shape: EyebrowShape::default(),
            eyelash_shape: EyelashShape::default(),
            iris_shape: IrisShape::default(),
            pupil_shape: PupilShape::default(),
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
    to.iris_shape = from.iris_shape.clone();
    to.pupil_shape = from.pupil_shape.clone();
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
) -> GuiActions {
    let mut actions = GuiActions::default();
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
                        let old_iris_radius = u.iris_radius;
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
                        let old_pupil_radius = u.pupil_radius;
                        ui.add(
                            egui::Slider::new(&mut u.pupil_radius, 0.01..=0.20)
                                .text("Pupil Radius"),
                        );

                        // Save radius values and detect changes before releasing the borrow on uniforms
                        let iris_radius_val = u.iris_radius;
                        let pupil_radius_val = u.pupil_radius;
                        let iris_radius_changed = (iris_radius_val - old_iris_radius).abs() > 1e-6;
                        let pupil_radius_changed = (pupil_radius_val - old_pupil_radius).abs() > 1e-6;

                        // --- Iris Shape Editor ---
                        ui.separator();
                        ui.label("Iris Shape");
                        let side_suffix = if link_iris.linked {
                            ""
                        } else if link_iris.active == Side::Left {
                            "_left"
                        } else {
                            "_right"
                        };
                        let iris_shape = if editing_left {
                            &mut left.iris_shape
                        } else {
                            &mut right.iris_shape
                        };
                        if iris_radius_changed {
                            iris_shape.outline = BezierOutline::circle(iris_radius_val);
                        }
                        let iris_editor_id = format!("iris_shape{side_suffix}");
                        bezier_outline_editor(ui, &mut iris_shape.outline, &iris_editor_id);
                        if ui.button("Reset Iris Circle").clicked() {
                            iris_shape.outline = BezierOutline::circle(iris_radius_val);
                        }

                        // --- Pupil Shape Editor ---
                        ui.separator();
                        ui.label("Pupil Shape");
                        let pupil_shape = if editing_left {
                            &mut left.pupil_shape
                        } else {
                            &mut right.pupil_shape
                        };
                        if pupil_radius_changed {
                            pupil_shape.outline = BezierOutline::circle(pupil_radius_val);
                        }
                        let pupil_editor_id = format!("pupil_shape{side_suffix}");
                        bezier_outline_editor(ui, &mut pupil_shape.outline, &pupil_editor_id);
                        if ui.button("Reset Pupil Circle").clicked() {
                            pupil_shape.outline = BezierOutline::circle(pupil_radius_val);
                        }

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
                        eyebrow_guide_outline_editor(ui, &mut eyebrow_shape.outline, &mut eyebrow_shape.guide, &editor_id);
                        // Tip thickness sliders: left tip = outline[0]/[5], right tip = outline[2]/[3]
                        for &(top_idx, bot_idx, label) in &[(0usize, 5usize, "Tip L"), (2usize, 3usize, "Tip R")] {
                            let top_pos = eyebrow_shape.outline.anchors[top_idx].position;
                            let bot_pos = eyebrow_shape.outline.anchors[bot_idx].position;
                            let mut thickness = ((top_pos[0] - bot_pos[0]).powi(2) + (top_pos[1] - bot_pos[1]).powi(2)).sqrt();
                            let old = thickness;
                            ui.add(
                                egui::Slider::new(&mut thickness, 0.001..=0.15).text(label),
                            );
                            if (thickness - old).abs() > 1e-6 && old > 1e-6 {
                                // Adjust top/bottom positions symmetrically around their midpoint
                                let mid = [
                                    (top_pos[0] + bot_pos[0]) * 0.5,
                                    (top_pos[1] + bot_pos[1]) * 0.5,
                                ];
                                let dir = [top_pos[0] - bot_pos[0], top_pos[1] - bot_pos[1]];
                                let dir_len = old;
                                let norm = [dir[0] / dir_len, dir[1] / dir_len];
                                let half = thickness * 0.5;
                                eyebrow_shape.outline.anchors[top_idx].position = [mid[0] + norm[0] * half, mid[1] + norm[1] * half];
                                eyebrow_shape.outline.anchors[bot_idx].position = [mid[0] - norm[0] * half, mid[1] - norm[1] * half];
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

                if ui.button("Export JSON").clicked() {
                    actions.export_requested = true;
                }
            });
        });
    actions
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

/// Compute the screen-space centroid of selected anchors.
fn centroid_screen(
    anchors: &[BezierAnchor; 4],
    selected: &[bool; 4],
    to_screen: &impl Fn([f32; 2]) -> egui::Pos2,
) -> egui::Pos2 {
    let mut sx = 0.0f32;
    let mut sy = 0.0f32;
    let mut n = 0u32;
    for i in 0..4 {
        if selected[i] {
            let scr = to_screen(anchors[i].position);
            sx += scr.x;
            sy += scr.y;
            n += 1;
        }
    }
    if n == 0 {
        egui::pos2(0.0, 0.0)
    } else {
        egui::pos2(sx / n as f32, sy / n as f32)
    }
}

/// Compute the eye-space centroid of selected anchors from snapshots.
fn centroid_eye_space(
    snaps: &[BezierAnchorSnapshot; 4],
    selected: &[bool; 4],
) -> [f32; 2] {
    let mut sx = 0.0f32;
    let mut sy = 0.0f32;
    let mut n = 0u32;
    for i in 0..4 {
        if selected[i] {
            sx += snaps[i].position[0];
            sy += snaps[i].position[1];
            n += 1;
        }
    }
    if n == 0 { [0.0, 0.0] } else { [sx / n as f32, sy / n as f32] }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum AxisConstraint {
    None,
    X,
    Y,
}

#[derive(Clone, Debug)]
enum BezierEditMode {
    Idle,
    Grab {
        /// Which anchors are being grabbed.
        selected: [bool; 4],
        original_anchors: [BezierAnchorSnapshot; 4],
        /// Mouse position (screen coords) at the moment G was pressed.
        grab_origin: [f32; 2],
    },
    Scale {
        /// Which anchors are being scaled.
        selected: [bool; 4],
        original_anchors: [BezierAnchorSnapshot; 4],
        /// Pivot point in screen coords (centroid of selected anchors).
        pivot_screen_pos: [f32; 2],
        initial_mouse_dist: f32,
        /// Axis constraint: None = uniform, X = X-only, Y = Y-only.
        axis: AxisConstraint,
    },
    Rotate {
        /// Which anchors are being rotated.
        selected: [bool; 4],
        original_anchors: [BezierAnchorSnapshot; 4],
        /// Pivot point in screen coords (centroid of selected anchors).
        pivot_screen_pos: [f32; 2],
        initial_mouse_angle: f32,
    },
}

#[derive(Clone, Debug)]
struct BezierEditorState {
    drag_idx: i32,
    /// Which anchors are selected (anchor-level selection).
    selected_anchors: [bool; 4],
    mode: BezierEditMode,
    /// Skip the next click-to-select (set after modal confirm via click).
    skip_click_select: bool,
    /// Box selection start position in screen coords. None = not active.
    box_select_origin: Option<[f32; 2]>,
}

impl Default for BezierEditorState {
    fn default() -> Self {
        Self {
            drag_idx: DRAG_NONE,
            selected_anchors: [false; 4],
            mode: BezierEditMode::Idle,
            skip_click_select: false,
            box_select_origin: None,
        }
    }
}

impl BezierEditorState {
    fn has_selection(&self) -> bool {
        self.selected_anchors.iter().any(|&s| s)
    }

    fn selection_count(&self) -> usize {
        self.selected_anchors.iter().filter(|&&s| s).count()
    }

    fn clear_selection(&mut self) {
        self.selected_anchors = [false; 4];
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
        let hi_active = hovered_idx == 4 + i as i32 || es.drag_idx == 4 + i as i32 || es.selected_anchors[i];
        let ho_active = hovered_idx == 8 + i as i32 || es.drag_idx == 8 + i as i32 || es.selected_anchors[i];
        painter.circle_filled(hi_scr, if hi_active { 5.0 } else { 3.5 }, if hi_active { handle_hover } else { handle_color });
        painter.circle_filled(ho_scr, if ho_active { 5.0 } else { 3.5 }, if ho_active { handle_hover } else { handle_color });

        // Selection rings for handles
        if es.selected_anchors[i] {
            painter.circle_stroke(hi_scr, 7.0, egui::Stroke::new(1.5, select_ring_color));
            painter.circle_stroke(ho_scr, 7.0, egui::Stroke::new(1.5, select_ring_color));
        }
    }

    // Draw anchor points (on top of everything)
    for i in 0..4 {
        let a_scr = to_screen(anchors[i].position);
        let active = hovered_idx == i as i32 || es.drag_idx == i as i32 || es.selected_anchors[i];
        painter.circle_filled(a_scr, if active { 7.0 } else { 5.0 }, if active { anchor_hover } else { anchor_color });

        // Selection ring for anchor
        if es.selected_anchors[i] {
            painter.circle_stroke(a_scr, 9.0, egui::Stroke::new(1.5, select_ring_color));
        }
    }

    // --- Centroid crosshair (when multiple anchors selected) ---
    if es.selection_count() > 1 {
        let centroid = centroid_screen(&outline.anchors, &es.selected_anchors, &to_screen);
        let cross_size = 6.0;
        let centroid_color = egui::Color32::from_rgb(255, 100, 100);
        painter.line_segment(
            [egui::pos2(centroid.x - cross_size, centroid.y),
             egui::pos2(centroid.x + cross_size, centroid.y)],
            egui::Stroke::new(1.5, centroid_color),
        );
        painter.line_segment(
            [egui::pos2(centroid.x, centroid.y - cross_size),
             egui::pos2(centroid.x, centroid.y + cross_size)],
            egui::Stroke::new(1.5, centroid_color),
        );
    }

    // --- Box selection overlay ---
    if let Some(origin) = es.box_select_origin {
        if let Some(pos) = response.hover_pos().or(response.interact_pointer_pos()) {
            let sel_rect = egui::Rect::from_two_pos(
                egui::pos2(origin[0], origin[1]),
                pos,
            );
            painter.rect_filled(
                sel_rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(100, 180, 255, 30),
            );
            let border_color = egui::Color32::from_rgba_unmultiplied(100, 180, 255, 150);
            let border_stroke = egui::Stroke::new(1.0, border_color);
            painter.line_segment([sel_rect.left_top(), sel_rect.right_top()], border_stroke);
            painter.line_segment([sel_rect.right_top(), sel_rect.right_bottom()], border_stroke);
            painter.line_segment([sel_rect.right_bottom(), sel_rect.left_bottom()], border_stroke);
            painter.line_segment([sel_rect.left_bottom(), sel_rect.left_top()], border_stroke);
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
        BezierEditMode::Scale { axis, .. } => {
            let label = match axis {
                AxisConstraint::None => "Scale (click=confirm, Esc=cancel)",
                AxisConstraint::X    => "Scale X (click=confirm, Esc=cancel)",
                AxisConstraint::Y    => "Scale Y (click=confirm, Esc=cancel)",
            };
            painter.text(
                egui::pos2(rect.left() + 8.0, rect.top() + 8.0),
                egui::Align2::LEFT_TOP,
                label,
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
            let mut clicked_anchor: Option<usize> = None;
            for i in 0..4 {
                let a = &outline.anchors[i];
                let d = pos.distance(to_screen(a.position));
                if d < best_dist {
                    best_dist = d;
                    clicked_anchor = Some(i);
                }
                let hi = [a.position[0] + a.handle_in[0], a.position[1] + a.handle_in[1]];
                let d = pos.distance(to_screen(hi));
                if d < best_dist {
                    best_dist = d;
                    clicked_anchor = Some(i);
                }
                let ho = [a.position[0] + a.handle_out[0], a.position[1] + a.handle_out[1]];
                let d = pos.distance(to_screen(ho));
                if d < best_dist {
                    best_dist = d;
                    clicked_anchor = Some(i);
                }
            }
            if let Some(ai) = clicked_anchor {
                if ui.input(|i| i.modifiers.shift) {
                    es.selected_anchors[ai] = !es.selected_anchors[ai];
                } else {
                    es.clear_selection();
                    es.selected_anchors[ai] = true;
                }
                response.request_focus();
            } else {
                es.clear_selection();
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

            // No point nearby -- begin box selection
            if es.drag_idx == DRAG_NONE {
                es.box_select_origin = Some([pos.x, pos.y]);
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

    // Request repaint during box selection drag
    if matches!(es.mode, BezierEditMode::Idle) && response.dragged() && es.box_select_origin.is_some() {
        ui.ctx().request_repaint();
    }

    if matches!(es.mode, BezierEditMode::Idle) && response.drag_stopped() {
        // Finalize box selection
        if let Some(origin) = es.box_select_origin.take() {
            if let Some(pos) = response.interact_pointer_pos() {
                let sel_rect = egui::Rect::from_two_pos(
                    egui::pos2(origin[0], origin[1]),
                    pos,
                );
                es.clear_selection();
                let mut any_selected = false;
                for i in 0..4 {
                    let scr = to_screen(outline.anchors[i].position);
                    if sel_rect.contains(scr) {
                        es.selected_anchors[i] = true;
                        any_selected = true;
                    }
                }
                if any_selected {
                    response.request_focus();
                }
            }
        }
        es.drag_idx = DRAG_NONE;
    }

    // --- Modal editing (G = Grab, S = Scale, R = Rotate, A = Select All) ---
    let has_focus = response.has_focus();
    match es.mode.clone() {
        BezierEditMode::Idle => {
            if has_focus && es.has_selection() {
                if ui.input(|i| i.key_pressed(egui::Key::G)) {
                    let mouse_pos = ui.input(|i| i.pointer.hover_pos())
                        .unwrap_or(egui::pos2(center.x, center.y));
                    es.mode = BezierEditMode::Grab {
                        selected: es.selected_anchors,
                        original_anchors: snapshot_all(&outline.anchors),
                        grab_origin: [mouse_pos.x, mouse_pos.y],
                    };
                    ui.ctx().request_repaint();
                } else if ui.input(|i| i.key_pressed(egui::Key::S)) {
                    let pivot = centroid_screen(&outline.anchors, &es.selected_anchors, &to_screen);
                    let mouse_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(pivot);
                    let initial_dist = pivot.distance(mouse_pos).max(1.0);
                    es.mode = BezierEditMode::Scale {
                        selected: es.selected_anchors,
                        original_anchors: snapshot_all(&outline.anchors),
                        pivot_screen_pos: [pivot.x, pivot.y],
                        initial_mouse_dist: initial_dist,
                        axis: AxisConstraint::None,
                    };
                    ui.ctx().request_repaint();
                } else if ui.input(|i| i.key_pressed(egui::Key::R)) {
                    let pivot = centroid_screen(&outline.anchors, &es.selected_anchors, &to_screen);
                    let mouse_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(pivot);
                    let initial_angle = (mouse_pos.y - pivot.y).atan2(mouse_pos.x - pivot.x);
                    es.mode = BezierEditMode::Rotate {
                        selected: es.selected_anchors,
                        original_anchors: snapshot_all(&outline.anchors),
                        pivot_screen_pos: [pivot.x, pivot.y],
                        initial_mouse_angle: initial_angle,
                    };
                    ui.ctx().request_repaint();
                } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    es.clear_selection();
                    response.surrender_focus();
                }
            }
            // A key: select all / deselect all (works with or without current selection)
            if has_focus && ui.input(|i| i.key_pressed(egui::Key::A)) {
                if es.has_selection() {
                    es.clear_selection();
                } else {
                    es.selected_anchors = [true; 4];
                }
                ui.ctx().request_repaint();
            }
        }
        BezierEditMode::Grab { selected, original_anchors, grab_origin } => {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let delta = from_screen(mouse_pos);
                let origin = from_screen(egui::pos2(grab_origin[0], grab_origin[1]));
                let dx = delta[0] - origin[0];
                let dy = delta[1] - origin[1];

                for i in 0..4 {
                    if selected[i] {
                        let orig = &original_anchors[i];
                        outline.anchors[i].position = [orig.position[0] + dx, orig.position[1] + dy];
                        outline.anchors[i].handle_in = orig.handle_in;
                        outline.anchors[i].handle_out = orig.handle_out;
                    }
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
        BezierEditMode::Scale { selected, original_anchors, pivot_screen_pos, initial_mouse_dist, mut axis } => {
            // Toggle axis constraint with X/Y keys
            if ui.input(|i| i.key_pressed(egui::Key::X)) {
                axis = if axis == AxisConstraint::X { AxisConstraint::None } else { AxisConstraint::X };
            }
            if ui.input(|i| i.key_pressed(egui::Key::Y)) {
                axis = if axis == AxisConstraint::Y { AxisConstraint::None } else { AxisConstraint::Y };
            }

            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let pivot_scr = egui::pos2(pivot_screen_pos[0], pivot_screen_pos[1]);
                let current_dist = pivot_scr.distance(mouse_pos).max(1.0);
                let scale_factor = current_dist / initial_mouse_dist;

                let (sx, sy) = match axis {
                    AxisConstraint::None => (scale_factor, scale_factor),
                    AxisConstraint::X    => (scale_factor, 1.0),
                    AxisConstraint::Y    => (1.0, scale_factor),
                };

                let centroid = centroid_eye_space(&original_anchors, &selected);

                for i in 0..4 {
                    if selected[i] {
                        let orig = &original_anchors[i];
                        outline.anchors[i].position = [
                            centroid[0] + (orig.position[0] - centroid[0]) * sx,
                            centroid[1] + (orig.position[1] - centroid[1]) * sy,
                        ];
                        outline.anchors[i].handle_in = [
                            orig.handle_in[0] * sx,
                            orig.handle_in[1] * sy,
                        ];
                        outline.anchors[i].handle_out = [
                            orig.handle_out[0] * sx,
                            orig.handle_out[1] * sy,
                        ];
                    }
                }
            }

            if ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) {
                es.mode = BezierEditMode::Idle;
                es.skip_click_select = true;
            } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                restore_all(&original_anchors, &mut outline.anchors);
                es.mode = BezierEditMode::Idle;
            } else {
                // Write back potentially updated axis
                es.mode = BezierEditMode::Scale {
                    selected, original_anchors, pivot_screen_pos, initial_mouse_dist, axis,
                };
            }
            ui.ctx().request_repaint();
        }
        BezierEditMode::Rotate { selected, original_anchors, pivot_screen_pos, initial_mouse_angle } => {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let pivot_scr = egui::pos2(pivot_screen_pos[0], pivot_screen_pos[1]);
                let current_angle = (mouse_pos.y - pivot_scr.y).atan2(mouse_pos.x - pivot_scr.x);
                let delta_angle = -(current_angle - initial_mouse_angle);
                let cos_a = delta_angle.cos();
                let sin_a = delta_angle.sin();

                let centroid = centroid_eye_space(&original_anchors, &selected);

                for i in 0..4 {
                    if selected[i] {
                        let orig = &original_anchors[i];
                        // Rotate position around centroid
                        let rel_x = orig.position[0] - centroid[0];
                        let rel_y = orig.position[1] - centroid[1];
                        outline.anchors[i].position = [
                            centroid[0] + rel_x * cos_a - rel_y * sin_a,
                            centroid[1] + rel_x * sin_a + rel_y * cos_a,
                        ];
                        // Rotate handles
                        outline.anchors[i].handle_in = [
                            orig.handle_in[0] * cos_a - orig.handle_in[1] * sin_a,
                            orig.handle_in[0] * sin_a + orig.handle_in[1] * cos_a,
                        ];
                        outline.anchors[i].handle_out = [
                            orig.handle_out[0] * cos_a - orig.handle_out[1] * sin_a,
                            orig.handle_out[0] * sin_a + orig.handle_out[1] * cos_a,
                        ];
                    }
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
    }

    ui.memory_mut(|m| m.data.insert_temp(state_id, es));
}

// ============================================================
// Eyebrow Bezier editor with guide curve + 6-point outline
// ============================================================

// Drag target encoding for eyebrow editor:
// Outline: 0-5 = anchor[i], 6-11 = handle_in[i-6], 12-17 = handle_out[i-12]
// Guide:  100-102 = guide anchor[i-100], 103-105 = guide handle_in[i-103], 106-108 = guide handle_out[i-106]
const EYEBROW_DRAG_NONE: i32 = -1;

#[derive(Clone, Debug)]
struct EyebrowAnchorSnapshot {
    position: [f32; 2],
    handle_in: [f32; 2],
    handle_out: [f32; 2],
}

impl EyebrowAnchorSnapshot {
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

fn snapshot_outline6(anchors: &[BezierAnchor; 6]) -> Vec<EyebrowAnchorSnapshot> {
    anchors.iter().map(EyebrowAnchorSnapshot::from_anchor).collect()
}

fn snapshot_guide3(anchors: &[BezierAnchor; 3]) -> Vec<EyebrowAnchorSnapshot> {
    anchors.iter().map(EyebrowAnchorSnapshot::from_anchor).collect()
}

fn restore_outline6(snaps: &[EyebrowAnchorSnapshot], anchors: &mut [BezierAnchor; 6]) {
    for (s, a) in snaps.iter().zip(anchors.iter_mut()) {
        s.restore_to(a);
    }
}

fn restore_guide3(snaps: &[EyebrowAnchorSnapshot], anchors: &mut [BezierAnchor; 3]) {
    for (s, a) in snaps.iter().zip(anchors.iter_mut()) {
        s.restore_to(a);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum EyebrowEditLayer {
    Outline,
    Guide,
}

#[derive(Clone, Debug)]
enum EyebrowEditMode {
    Idle,
    Grab {
        layer: EyebrowEditLayer,
        /// For outline: [bool; 6], for guide: [bool; 3]
        selected: Vec<bool>,
        original_outline: Vec<EyebrowAnchorSnapshot>,
        original_guide: Vec<EyebrowAnchorSnapshot>,
        grab_origin: [f32; 2],
    },
}

#[derive(Clone, Debug)]
struct EyebrowEditorState {
    drag_idx: i32,
    /// Outline anchor selection [bool; 6]
    outline_selected: Vec<bool>,
    /// Guide anchor selection [bool; 3]
    guide_selected: Vec<bool>,
    active_layer: EyebrowEditLayer,
    mode: EyebrowEditMode,
    skip_click_select: bool,
    box_select_origin: Option<[f32; 2]>,
}

impl Default for EyebrowEditorState {
    fn default() -> Self {
        Self {
            drag_idx: EYEBROW_DRAG_NONE,
            outline_selected: vec![false; 6],
            guide_selected: vec![false; 3],
            active_layer: EyebrowEditLayer::Outline,
            mode: EyebrowEditMode::Idle,
            skip_click_select: false,
            box_select_origin: None,
        }
    }
}

impl EyebrowEditorState {
    fn has_outline_selection(&self) -> bool {
        self.outline_selected.iter().any(|&s| s)
    }

    fn has_guide_selection(&self) -> bool {
        self.guide_selected.iter().any(|&s| s)
    }

    fn has_any_selection(&self) -> bool {
        self.has_outline_selection() || self.has_guide_selection()
    }

    fn clear_all_selection(&mut self) {
        for s in &mut self.outline_selected { *s = false; }
        for s in &mut self.guide_selected { *s = false; }
    }
}

fn eyebrow_guide_outline_editor(
    ui: &mut egui::Ui,
    outline: &mut EyebrowOutline,
    guide: &mut EyebrowGuide,
    editor_id: &str,
) {
    let available_width = ui.available_width();
    let size = available_width.min(350.0);
    let (response, painter) = ui.allocate_painter(
        egui::vec2(size, size * 0.6),
        egui::Sense::click_and_drag() | egui::Sense::FOCUSABLE,
    );
    let rect = response.rect;

    // Eyebrow coords span roughly ±0.3 in x and ±0.1 in y,
    // so use a larger scale than the generic eye editor.
    let scale = rect.width() * 1.6;
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
    let state_id = response.id.with(editor_id).with("eyebrow_editor_state");
    let mut es: EyebrowEditorState =
        ui.memory(|m| m.data.get_temp(state_id)).unwrap_or_default();

    // Ensure correct lengths (in case of stale data)
    if es.outline_selected.len() != 6 { es.outline_selected = vec![false; 6]; }
    if es.guide_selected.len() != 3 { es.guide_selected = vec![false; 3]; }

    // --- Find hovered element ---
    let hover_threshold = 12.0f32;
    let mut hovered_idx: i32 = EYEBROW_DRAG_NONE;
    if es.drag_idx == EYEBROW_DRAG_NONE && matches!(es.mode, EyebrowEditMode::Idle) {
        if let Some(pos) = response.hover_pos() {
            let mut best_dist = hover_threshold;
            // Check outline anchors (0-5) and handles (6-11, 12-17)
            for i in 0..6 {
                let a = &outline.anchors[i];
                let d = pos.distance(to_screen(a.position));
                if d < best_dist { best_dist = d; hovered_idx = i as i32; }
                let hi = extend_handle(a.position, a.handle_in);
                let d = pos.distance(to_screen(hi));
                if d < best_dist { best_dist = d; hovered_idx = 6 + i as i32; }
                let ho = extend_handle(a.position, a.handle_out);
                let d = pos.distance(to_screen(ho));
                if d < best_dist { best_dist = d; hovered_idx = 12 + i as i32; }
            }
            // Check guide anchors (100-102) and handles (103-105, 106-108)
            for i in 0..3 {
                let a = &guide.anchors[i];
                let d = pos.distance(to_screen(a.position));
                if d < best_dist { best_dist = d; hovered_idx = 100 + i as i32; }
                // Only check handles for middle point (endpoints have in/out only for their segments)
                let hi = extend_handle(a.position, a.handle_in);
                let d = pos.distance(to_screen(hi));
                if d < best_dist { best_dist = d; hovered_idx = 103 + i as i32; }
                let ho = extend_handle(a.position, a.handle_out);
                let d = pos.distance(to_screen(ho));
                if d < best_dist { best_dist = d; hovered_idx = 106 + i as i32; }
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

    // --- Colors ---
    let outline_curve_color = egui::Color32::from_rgb(220, 80, 80);
    let guide_curve_color = egui::Color32::from_rgb(80, 120, 220);

    let outline_handle_line_color = egui::Color32::from_gray(100);
    let outline_handle_color = egui::Color32::from_rgb(255, 160, 0);
    let outline_handle_hover = egui::Color32::from_rgb(255, 220, 100);
    let outline_anchor_color = egui::Color32::WHITE;
    let outline_anchor_hover = egui::Color32::from_rgb(255, 255, 180);

    let guide_handle_line_color = egui::Color32::from_rgb(60, 80, 140);
    let guide_handle_color = egui::Color32::from_rgb(100, 140, 220);
    let guide_handle_hover = egui::Color32::from_rgb(140, 180, 255);
    let guide_anchor_color = egui::Color32::from_rgb(60, 100, 200);
    let guide_anchor_hover = egui::Color32::from_rgb(100, 160, 255);

    let select_ring_color = egui::Color32::from_rgb(100, 180, 255);

    // --- Draw outline curve (red, 6 segments closed) ---
    for seg in 0..6 {
        let next = (seg + 1) % 6;
        let a = &outline.anchors[seg];
        let b = &outline.anchors[next];
        let p0 = a.position;
        let p1 = [p0[0] + a.handle_out[0], p0[1] + a.handle_out[1]];
        let p3 = b.position;
        let p2 = [p3[0] + b.handle_in[0], p3[1] + b.handle_in[1]];

        let subdiv = 24;
        let mut prev_pt = to_screen(p0);
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
            painter.line_segment([prev_pt, curr], egui::Stroke::new(2.0, outline_curve_color));
            prev_pt = curr;
        }
    }

    // --- Draw guide curve (blue, 2 segments open) ---
    for seg in 0..2 {
        let a = &guide.anchors[seg];
        let b = &guide.anchors[seg + 1];
        let p0 = a.position;
        let p1 = [p0[0] + a.handle_out[0], p0[1] + a.handle_out[1]];
        let p3 = b.position;
        let p2 = [p3[0] + b.handle_in[0], p3[1] + b.handle_in[1]];

        let subdiv = 24;
        let mut prev_pt = to_screen(p0);
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
            painter.line_segment([prev_pt, curr], egui::Stroke::new(1.5, guide_curve_color));
            prev_pt = curr;
        }
    }

    // --- Draw outline handles and anchors ---
    for i in 0..6 {
        let a = &outline.anchors[i];
        let hi = extend_handle(a.position, a.handle_in);
        let ho = extend_handle(a.position, a.handle_out);
        let hi_scr = to_screen(hi);
        let ho_scr = to_screen(ho);

        painter.line_segment([hi_scr, ho_scr], egui::Stroke::new(1.0, outline_handle_line_color));

        let hi_active = hovered_idx == 6 + i as i32 || es.drag_idx == 6 + i as i32 || es.outline_selected[i];
        let ho_active = hovered_idx == 12 + i as i32 || es.drag_idx == 12 + i as i32 || es.outline_selected[i];
        painter.circle_filled(hi_scr, if hi_active { 5.0 } else { 3.5 }, if hi_active { outline_handle_hover } else { outline_handle_color });
        painter.circle_filled(ho_scr, if ho_active { 5.0 } else { 3.5 }, if ho_active { outline_handle_hover } else { outline_handle_color });

        if es.outline_selected[i] {
            painter.circle_stroke(hi_scr, 7.0, egui::Stroke::new(1.5, select_ring_color));
            painter.circle_stroke(ho_scr, 7.0, egui::Stroke::new(1.5, select_ring_color));
        }
    }

    for i in 0..6 {
        let a_scr = to_screen(outline.anchors[i].position);
        let active = hovered_idx == i as i32 || es.drag_idx == i as i32 || es.outline_selected[i];
        painter.circle_filled(a_scr, if active { 7.0 } else { 5.0 }, if active { outline_anchor_hover } else { outline_anchor_color });
        if es.outline_selected[i] {
            painter.circle_stroke(a_scr, 9.0, egui::Stroke::new(1.5, select_ring_color));
        }
    }

    // --- Draw guide handles and anchors ---
    for i in 0..3 {
        let a = &guide.anchors[i];
        let hi = extend_handle(a.position, a.handle_in);
        let ho = extend_handle(a.position, a.handle_out);
        let hi_scr = to_screen(hi);
        let ho_scr = to_screen(ho);

        painter.line_segment([hi_scr, ho_scr], egui::Stroke::new(1.0, guide_handle_line_color));

        let hi_active = hovered_idx == 103 + i as i32 || es.drag_idx == 103 + i as i32 || es.guide_selected[i];
        let ho_active = hovered_idx == 106 + i as i32 || es.drag_idx == 106 + i as i32 || es.guide_selected[i];
        painter.circle_filled(hi_scr, if hi_active { 5.0 } else { 3.5 }, if hi_active { guide_handle_hover } else { guide_handle_color });
        painter.circle_filled(ho_scr, if ho_active { 5.0 } else { 3.5 }, if ho_active { guide_handle_hover } else { guide_handle_color });

        if es.guide_selected[i] {
            painter.circle_stroke(hi_scr, 7.0, egui::Stroke::new(1.5, select_ring_color));
            painter.circle_stroke(ho_scr, 7.0, egui::Stroke::new(1.5, select_ring_color));
        }
    }

    for i in 0..3 {
        let a_scr = to_screen(guide.anchors[i].position);
        let active = hovered_idx == 100 + i as i32 || es.drag_idx == 100 + i as i32 || es.guide_selected[i];
        painter.circle_filled(a_scr, if active { 7.0 } else { 5.0 }, if active { guide_anchor_hover } else { guide_anchor_color });
        if es.guide_selected[i] {
            painter.circle_stroke(a_scr, 9.0, egui::Stroke::new(1.5, select_ring_color));
        }
    }

    // --- Mode indicator ---
    match &es.mode {
        EyebrowEditMode::Grab { layer, .. } => {
            let label = match layer {
                EyebrowEditLayer::Guide => "Grab Guide (click=confirm, Esc=cancel)",
                EyebrowEditLayer::Outline => "Grab Outline (click=confirm, Esc=cancel)",
            };
            painter.text(
                egui::pos2(rect.left() + 8.0, rect.top() + 8.0),
                egui::Align2::LEFT_TOP,
                label,
                egui::FontId::proportional(11.0),
                select_ring_color,
            );
        }
        EyebrowEditMode::Idle => {}
    }

    // --- Layer indicator ---
    let layer_label = match es.active_layer {
        EyebrowEditLayer::Outline => "Outline",
        EyebrowEditLayer::Guide => "Guide",
    };
    painter.text(
        egui::pos2(rect.right() - 8.0, rect.top() + 8.0),
        egui::Align2::RIGHT_TOP,
        layer_label,
        egui::FontId::proportional(10.0),
        egui::Color32::from_gray(120),
    );

    // --- Click-to-select ---
    if matches!(es.mode, EyebrowEditMode::Idle) && response.clicked() {
        if es.skip_click_select {
            es.skip_click_select = false;
        } else if let Some(pos) = response.interact_pointer_pos() {
            let threshold = 15.0f32;
            let mut best_dist = threshold;
            let mut clicked_outline: Option<usize> = None;
            let mut clicked_guide: Option<usize> = None;

            // Check outline points
            for i in 0..6 {
                let a = &outline.anchors[i];
                let d = pos.distance(to_screen(a.position));
                if d < best_dist { best_dist = d; clicked_outline = Some(i); clicked_guide = None; }
                let hi = extend_handle(a.position, a.handle_in);
                let d = pos.distance(to_screen(hi));
                if d < best_dist { best_dist = d; clicked_outline = Some(i); clicked_guide = None; }
                let ho = extend_handle(a.position, a.handle_out);
                let d = pos.distance(to_screen(ho));
                if d < best_dist { best_dist = d; clicked_outline = Some(i); clicked_guide = None; }
            }
            // Check guide points
            for i in 0..3 {
                let a = &guide.anchors[i];
                let d = pos.distance(to_screen(a.position));
                if d < best_dist { best_dist = d; clicked_guide = Some(i); clicked_outline = None; }
                let hi = extend_handle(a.position, a.handle_in);
                let d = pos.distance(to_screen(hi));
                if d < best_dist { best_dist = d; clicked_guide = Some(i); clicked_outline = None; }
                let ho = extend_handle(a.position, a.handle_out);
                let d = pos.distance(to_screen(ho));
                if d < best_dist { best_dist = d; clicked_guide = Some(i); clicked_outline = None; }
            }

            if let Some(ai) = clicked_outline {
                if !ui.input(|i| i.modifiers.shift) { es.clear_all_selection(); }
                es.outline_selected[ai] = !es.outline_selected[ai];
                es.active_layer = EyebrowEditLayer::Outline;
                response.request_focus();
            } else if let Some(gi) = clicked_guide {
                if !ui.input(|i| i.modifiers.shift) { es.clear_all_selection(); }
                es.guide_selected[gi] = !es.guide_selected[gi];
                es.active_layer = EyebrowEditLayer::Guide;
                response.request_focus();
            } else {
                es.clear_all_selection();
            }
        }
    }

    // --- Drag interaction ---
    if matches!(es.mode, EyebrowEditMode::Idle) && response.drag_started() {
        if let Some(pos) = response.interact_pointer_pos() {
            let threshold = 15.0f32;
            let mut best_dist = threshold;
            es.drag_idx = EYEBROW_DRAG_NONE;

            for i in 0..6 {
                let a = &outline.anchors[i];
                let d = pos.distance(to_screen(a.position));
                if d < best_dist { best_dist = d; es.drag_idx = i as i32; }
                let hi = extend_handle(a.position, a.handle_in);
                let d = pos.distance(to_screen(hi));
                if d < best_dist { best_dist = d; es.drag_idx = 6 + i as i32; }
                let ho = extend_handle(a.position, a.handle_out);
                let d = pos.distance(to_screen(ho));
                if d < best_dist { best_dist = d; es.drag_idx = 12 + i as i32; }
            }
            for i in 0..3 {
                let a = &guide.anchors[i];
                let d = pos.distance(to_screen(a.position));
                if d < best_dist { best_dist = d; es.drag_idx = 100 + i as i32; }
                let hi = extend_handle(a.position, a.handle_in);
                let d = pos.distance(to_screen(hi));
                if d < best_dist { best_dist = d; es.drag_idx = 103 + i as i32; }
                let ho = extend_handle(a.position, a.handle_out);
                let d = pos.distance(to_screen(ho));
                if d < best_dist { best_dist = d; es.drag_idx = 106 + i as i32; }
            }

            if es.drag_idx == EYEBROW_DRAG_NONE {
                es.box_select_origin = Some([pos.x, pos.y]);
            }
        }
    }

    if matches!(es.mode, EyebrowEditMode::Idle) && response.dragged() && es.drag_idx != EYEBROW_DRAG_NONE {
        if let Some(pos) = response.interact_pointer_pos() {
            let p = from_screen(pos);
            let idx = es.drag_idx;

            if idx < 6 {
                // Outline anchor drag
                let i = idx as usize;
                outline.anchors[i].position = p;
            } else if idx < 12 {
                // Outline handle_in drag
                let i = (idx - 6) as usize;
                let anchor = outline.anchors[i].position;
                outline.anchors[i].handle_in = [p[0] - anchor[0], p[1] - anchor[1]];
                outline.anchors[i].enforce_collinear_from_in();
            } else if idx < 18 {
                // Outline handle_out drag
                let i = (idx - 12) as usize;
                let anchor = outline.anchors[i].position;
                outline.anchors[i].handle_out = [p[0] - anchor[0], p[1] - anchor[1]];
                outline.anchors[i].enforce_collinear_from_out();
            } else if idx >= 100 && idx < 103 {
                // Guide anchor drag → propagate to outline
                let gi = (idx - 100) as usize;
                let old_pos = guide.anchors[gi].position;
                guide.anchors[gi].position = p;
                let delta = [p[0] - old_pos[0], p[1] - old_pos[1]];
                EyebrowGuide::propagate_delta(gi, delta, outline);
            } else if idx >= 103 && idx < 106 {
                // Guide handle_in drag
                let gi = (idx - 103) as usize;
                let anchor = guide.anchors[gi].position;
                guide.anchors[gi].handle_in = [p[0] - anchor[0], p[1] - anchor[1]];
                guide.anchors[gi].enforce_collinear_from_in();
            } else if idx >= 106 && idx < 109 {
                // Guide handle_out drag
                let gi = (idx - 106) as usize;
                let anchor = guide.anchors[gi].position;
                guide.anchors[gi].handle_out = [p[0] - anchor[0], p[1] - anchor[1]];
                guide.anchors[gi].enforce_collinear_from_out();
            }
        }
    }

    // Box selection repaint
    if matches!(es.mode, EyebrowEditMode::Idle) && response.dragged() && es.box_select_origin.is_some() {
        ui.ctx().request_repaint();
    }

    // Box selection overlay
    if let Some(origin) = es.box_select_origin {
        if let Some(pos) = response.hover_pos().or(response.interact_pointer_pos()) {
            let sel_rect = egui::Rect::from_two_pos(
                egui::pos2(origin[0], origin[1]),
                pos,
            );
            painter.rect_filled(sel_rect, 0.0, egui::Color32::from_rgba_unmultiplied(100, 180, 255, 30));
            let border_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(100, 180, 255, 150));
            painter.line_segment([sel_rect.left_top(), sel_rect.right_top()], border_stroke);
            painter.line_segment([sel_rect.right_top(), sel_rect.right_bottom()], border_stroke);
            painter.line_segment([sel_rect.right_bottom(), sel_rect.left_bottom()], border_stroke);
            painter.line_segment([sel_rect.left_bottom(), sel_rect.left_top()], border_stroke);
        }
    }

    if matches!(es.mode, EyebrowEditMode::Idle) && response.drag_stopped() {
        if let Some(origin) = es.box_select_origin.take() {
            if let Some(pos) = response.interact_pointer_pos() {
                let sel_rect = egui::Rect::from_two_pos(egui::pos2(origin[0], origin[1]), pos);
                es.clear_all_selection();
                let mut any = false;
                for i in 0..6 {
                    if sel_rect.contains(to_screen(outline.anchors[i].position)) {
                        es.outline_selected[i] = true;
                        any = true;
                    }
                }
                for i in 0..3 {
                    if sel_rect.contains(to_screen(guide.anchors[i].position)) {
                        es.guide_selected[i] = true;
                        any = true;
                    }
                }
                if any { response.request_focus(); }
            }
        }
        es.drag_idx = EYEBROW_DRAG_NONE;
    }

    // --- Modal editing (G = Grab, A = Select All, Tab = switch layer) ---
    let has_focus = response.has_focus();
    match es.mode.clone() {
        EyebrowEditMode::Idle => {
            // Tab: switch active layer
            if has_focus && ui.input(|i| i.key_pressed(egui::Key::Tab)) {
                es.active_layer = match es.active_layer {
                    EyebrowEditLayer::Outline => EyebrowEditLayer::Guide,
                    EyebrowEditLayer::Guide => EyebrowEditLayer::Outline,
                };
                ui.ctx().request_repaint();
            }
            // G: grab selected
            if has_focus && es.has_any_selection() && ui.input(|i| i.key_pressed(egui::Key::G)) {
                let mouse_pos = ui.input(|i| i.pointer.hover_pos())
                    .unwrap_or(egui::pos2(center.x, center.y));
                // Determine which layer is being grabbed
                let layer = if es.has_guide_selection() && !es.has_outline_selection() {
                    EyebrowEditLayer::Guide
                } else {
                    EyebrowEditLayer::Outline
                };
                let selected = match layer {
                    EyebrowEditLayer::Outline => es.outline_selected.clone(),
                    EyebrowEditLayer::Guide => es.guide_selected.clone(),
                };
                es.mode = EyebrowEditMode::Grab {
                    layer,
                    selected,
                    original_outline: snapshot_outline6(&outline.anchors),
                    original_guide: snapshot_guide3(&guide.anchors),
                    grab_origin: [mouse_pos.x, mouse_pos.y],
                };
                ui.ctx().request_repaint();
            }
            // A: select all / deselect all (for active layer)
            if has_focus && ui.input(|i| i.key_pressed(egui::Key::A)) {
                match es.active_layer {
                    EyebrowEditLayer::Outline => {
                        if es.has_outline_selection() {
                            for s in &mut es.outline_selected { *s = false; }
                        } else {
                            for s in &mut es.outline_selected { *s = true; }
                        }
                    }
                    EyebrowEditLayer::Guide => {
                        if es.has_guide_selection() {
                            for s in &mut es.guide_selected { *s = false; }
                        } else {
                            for s in &mut es.guide_selected { *s = true; }
                        }
                    }
                }
                ui.ctx().request_repaint();
            }
            // Escape: deselect
            if has_focus && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                es.clear_all_selection();
                response.surrender_focus();
            }
        }
        EyebrowEditMode::Grab { layer, selected, original_outline, original_guide, grab_origin } => {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let delta = from_screen(mouse_pos);
                let origin = from_screen(egui::pos2(grab_origin[0], grab_origin[1]));
                let dx = delta[0] - origin[0];
                let dy = delta[1] - origin[1];

                match layer {
                    EyebrowEditLayer::Outline => {
                        // Restore first, then apply delta to selected
                        restore_outline6(&original_outline, &mut outline.anchors);
                        for i in 0..6 {
                            if i < selected.len() && selected[i] {
                                outline.anchors[i].position[0] += dx;
                                outline.anchors[i].position[1] += dy;
                            }
                        }
                    }
                    EyebrowEditLayer::Guide => {
                        // Restore both guide and outline first
                        restore_guide3(&original_guide, &mut guide.anchors);
                        restore_outline6(&original_outline, &mut outline.anchors);
                        for gi in 0..3 {
                            if gi < selected.len() && selected[gi] {
                                guide.anchors[gi].position[0] += dx;
                                guide.anchors[gi].position[1] += dy;
                                // Propagate to paired outline points
                                EyebrowGuide::propagate_delta(gi, [dx, dy], outline);
                            }
                        }
                    }
                }
            }

            if ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) {
                es.mode = EyebrowEditMode::Idle;
                es.skip_click_select = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                restore_outline6(&original_outline, &mut outline.anchors);
                restore_guide3(&original_guide, &mut guide.anchors);
                es.mode = EyebrowEditMode::Idle;
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
    s.push_str("    outline: EyebrowOutline {\n        anchors: [\n");
    let labels = ["T0 (left)", "T1 (top)", "T2 (right)", "B0 (right)", "B1 (bottom)", "B2 (left)"];
    for (i, a) in shape.outline.anchors.iter().enumerate() {
        s.push_str(&format!("            // {}\n", labels[i]));
        s.push_str("            BezierAnchor {\n");
        s.push_str(&format!("                position: [{:.6}, {:.6}],\n", a.position[0], a.position[1]));
        s.push_str(&format!("                handle_in: [{:.6}, {:.6}],\n", a.handle_in[0], a.handle_in[1]));
        s.push_str(&format!("                handle_out: [{:.6}, {:.6}],\n", a.handle_out[0], a.handle_out[1]));
        s.push_str("            },\n");
    }
    s.push_str("        ],\n    },\n");
    s.push_str("    guide: EyebrowGuide {\n        anchors: [\n");
    let glabels = ["G0 (left)", "G1 (center)", "G2 (right)"];
    for (i, a) in shape.guide.anchors.iter().enumerate() {
        s.push_str(&format!("            // {}\n", glabels[i]));
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
