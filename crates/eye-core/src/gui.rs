use egui;

use crate::EyeUniforms;

pub fn eye_control_panel(ctx: &egui::Context, uniforms: &mut EyeUniforms) {
    egui::SidePanel::right("eye_controls")
        .default_width(280.0)
        .show(ctx, |ui| {
            ui.heading("Eye Controls");
            ui.separator();

            ui.add(
                egui::Slider::new(&mut uniforms.eyelid_close, 0.0..=1.0).text("Eyelid Close"),
            );

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

            if ui.button("Reset").clicked() {
                let aspect = uniforms.aspect_ratio;
                let time = uniforms.time;
                *uniforms = EyeUniforms::default();
                uniforms.aspect_ratio = aspect;
                uniforms.time = time;
            }
        });
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
