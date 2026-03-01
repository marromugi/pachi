use std::sync::Arc;
use std::time::Instant;

use eye::gui::{eye_control_panel, EyeSideState, GuiActions, SectionLink};
use eye::{BlinkAnimation, EyeConfig, EyePairUniforms, EyeRenderer};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

struct App {
    state: Option<AppState>,
    config_path: Option<String>,
}

struct AppState {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    renderer: EyeRenderer,

    // Per-eye state
    left: EyeSideState,
    right: EyeSideState,

    // Section link state
    link_shape: SectionLink,
    link_iris: SectionLink,
    link_eyebrow: SectionLink,
    link_eyelash: SectionLink,

    blink_animation: BlinkAnimation,
    auto_blink: bool,
    follow_mouse: bool,
    show_highlight: bool,
    show_eyebrow: bool,
    show_eyelash: bool,
    show_sidebar: bool,
    focus_distance: f32,
    mouse_position: Option<winit::dpi::PhysicalPosition<f64>>,
    start_time: Instant,

    // egui
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Eye")
                        .with_inner_size(winit::dpi::LogicalSize::new(1280, 720)),
                )
                .unwrap(),
        );

        let mut state = pollster::block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                ..Default::default()
            });

            let surface = instance.create_surface(window.clone()).unwrap();

            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                })
                .await
                .unwrap();

            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("eye_device"),
                        ..Default::default()
                    },
                    None,
                )
                .await
                .unwrap();

            let size = window.inner_size();
            let caps = surface.get_capabilities(&adapter);
            let format = caps.formats[0];

            let surface_config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.width.max(1),
                height: size.height.max(1),
                present_mode: wgpu::PresentMode::AutoVsync,
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&device, &surface_config);

            let renderer = EyeRenderer::new(&device, format);

            // egui setup
            let egui_ctx = egui::Context::default();
            let egui_state = egui_winit::State::new(
                egui_ctx.clone(),
                egui_ctx.viewport_id(),
                &window,
                Some(window.scale_factor() as f32),
                None,
                None,
            );
            let egui_renderer = egui_wgpu::Renderer::new(&device, format, None, 1, false);

            AppState {
                window,
                device,
                queue,
                surface,
                surface_config,
                renderer,
                left: EyeSideState::default(),
                right: EyeSideState::default(),
                link_shape: SectionLink::default(),
                link_iris: SectionLink::default(),
                link_eyebrow: SectionLink::default(),
                link_eyelash: SectionLink::default(),
                blink_animation: BlinkAnimation::sample(),
                auto_blink: true,
                follow_mouse: true,
                show_highlight: true,
                show_eyebrow: true,
                show_eyelash: true,
                show_sidebar: true,
                focus_distance: 1.5,
                mouse_position: None,
                start_time: Instant::now(),
                egui_ctx,
                egui_state,
                egui_renderer,
            }
        });

        // Apply config from command-line argument if provided
        if let Some(path) = &self.config_path {
            match std::fs::read_to_string(path) {
                Ok(json) => match EyeConfig::from_json(&json) {
                    Ok(config) => {
                        config.apply_to_state(
                            &mut state.left,
                            &mut state.right,
                            &mut state.link_shape,
                            &mut state.link_iris,
                            &mut state.link_eyebrow,
                            &mut state.link_eyelash,
                            &mut state.auto_blink,
                            &mut state.follow_mouse,
                            &mut state.show_highlight,
                            &mut state.show_eyebrow,
                            &mut state.show_eyelash,
                            &mut state.focus_distance,
                        );
                    }
                    Err(e) => eprintln!("Invalid config JSON: {e}"),
                },
                Err(e) => eprintln!("Failed to read config file: {e}"),
            }
        }

        self.state = Some(state);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = &mut self.state else {
            return;
        };

        // Intercept Tab before egui consumes it
        if let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    logical_key: Key::Named(NamedKey::Tab),
                    state: ElementState::Pressed,
                    ..
                },
            ..
        } = &event
        {
            state.show_sidebar = !state.show_sidebar;
            state.window.request_redraw();
            return;
        }

        // Pass events to egui first
        let egui_response = state.egui_state.on_window_event(&state.window, &event);
        if egui_response.consumed {
            state.window.request_redraw();
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                state.surface_config.width = new_size.width.max(1);
                state.surface_config.height = new_size.height.max(1);
                state
                    .surface
                    .configure(&state.device, &state.surface_config);
                state.window.request_redraw();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                event_loop.exit();
            }
            WindowEvent::CursorMoved { position, .. } => {
                state.mouse_position = Some(position);
                if state.follow_mouse {
                    state.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let output = match state.surface.get_current_texture() {
                    Ok(output) => output,
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        state
                            .surface
                            .configure(&state.device, &state.surface_config);
                        return;
                    }
                    Err(e) => {
                        eprintln!("Surface error: {e:?}");
                        return;
                    }
                };

                // Update dynamic uniforms (global: sync to both eyes)
                let aspect =
                    state.surface_config.width as f32 / state.surface_config.height as f32;
                let time = state.start_time.elapsed().as_secs_f32();
                state.left.uniforms.aspect_ratio = aspect;
                state.left.uniforms.time = time;
                state.right.uniforms.aspect_ratio = aspect;
                state.right.uniforms.time = time;

                // Auto-blink: applies to both eyes
                if state.auto_blink {
                    let eyelid_now = state.blink_animation.evaluate(time);

                    // Squash & stretch driven by eyelid velocity
                    let dt = 1.0 / 60.0_f32;
                    let eyelid_prev = state.blink_animation.evaluate(time - dt);
                    let velocity = (eyelid_now - eyelid_prev) / dt;
                    const SQUASH_STRENGTH: f32 = 0.08;
                    const MAX_SQUASH: f32 = 0.045;
                    let squash =
                        (velocity * SQUASH_STRENGTH).clamp(-MAX_SQUASH, MAX_SQUASH);

                    state.left.uniforms.squash_stretch = squash;
                    state.right.uniforms.squash_stretch = squash;
                    state.left.uniforms.eyelid_close = eyelid_now;
                    state.right.uniforms.eyelid_close = eyelid_now;
                } else {
                    state.left.uniforms.squash_stretch = 0.0;
                    state.right.uniforms.squash_stretch = 0.0;
                }

                // Mouse follow → look_x / look_y (applies to both eyes)
                if state.follow_mouse {
                    if let Some(pos) = state.mouse_position {
                        let cx = state.surface_config.width as f64 / 2.0;
                        let cy = state.surface_config.height as f64 / 2.0;
                        let look_x =
                            ((pos.x - cx) / cx).clamp(-1.0, 1.0) as f32;
                        let look_y =
                            -((pos.y - cy) / cy).clamp(-1.0, 1.0) as f32;
                        state.left.uniforms.look_x = look_x;
                        state.left.uniforms.look_y = look_y;
                        state.right.uniforms.look_x = look_x;
                        state.right.uniforms.look_y = look_y;
                    }
                }

                // Focus distance → convergence offset (global)
                let half_ipd = state.left.uniforms.eye_separation * 0.5;
                let convergence = (half_ipd / state.focus_distance * 0.08)
                    .clamp(0.0, state.left.uniforms.iris_follow * 0.8);
                state.left.uniforms.convergence = convergence;
                state.right.uniforms.convergence = convergence;

                // Sync shapes into respective uniforms
                state.left.uniforms.outline_open =
                    state.left.eye_shape.open.to_uniform_array();
                state.left.uniforms.outline_closed =
                    state.left.eye_shape.closed.to_uniform_array();
                state.right.uniforms.outline_open =
                    state.right.eye_shape.open.to_uniform_array();
                state.right.uniforms.outline_closed =
                    state.right.eye_shape.closed.to_uniform_array();

                // Sync eyebrow shapes into uniforms
                state.left.uniforms.eyebrow_color = state.left.eyebrow_shape.color;
                state.left.uniforms.eyebrow_base_y = state.left.eyebrow_shape.base_y;
                state.left.uniforms.eyebrow_follow = state.left.eyebrow_shape.follow;
                state.left.uniforms.eyebrow_outline =
                    state.left.eyebrow_shape.outline.to_uniform_array();
                state.right.uniforms.eyebrow_color = state.right.eyebrow_shape.color;
                state.right.uniforms.eyebrow_base_y = state.right.eyebrow_shape.base_y;
                state.right.uniforms.eyebrow_follow = state.right.eyebrow_shape.follow;
                state.right.uniforms.eyebrow_outline =
                    state.right.eyebrow_shape.outline.to_uniform_array();

                // Sync eyelash shapes into uniforms
                state.left.uniforms.eyelash_color = state.left.eyelash_shape.color;
                state.left.uniforms.eyelash_thickness = state.left.eyelash_shape.thickness;
                state.right.uniforms.eyelash_color = state.right.eyelash_shape.color;
                state.right.uniforms.eyelash_thickness = state.right.eyelash_shape.thickness;

                // Sync iris/pupil shapes into uniforms
                state.left.uniforms.iris_outline =
                    state.left.iris_shape.outline.to_uniform_array();
                state.right.uniforms.iris_outline =
                    state.right.iris_shape.outline.to_uniform_array();
                state.left.uniforms.pupil_outline =
                    state.left.pupil_shape.outline.to_uniform_array();
                state.right.uniforms.pupil_outline =
                    state.right.pupil_shape.outline.to_uniform_array();

                // Sync global params left → right
                state.right.uniforms.bg_color = state.left.uniforms.bg_color;
                state.right.uniforms.eye_separation = state.left.uniforms.eye_separation;
                state.right.uniforms.max_angle = state.left.uniforms.max_angle;
                state.right.uniforms.eye_angle = state.left.uniforms.eye_angle;

                // --- egui frame ---
                let raw_input = state.egui_state.take_egui_input(&state.window);
                let show_sidebar = state.show_sidebar;
                let mut gui_actions = GuiActions::default();
                let full_output = state.egui_ctx.run(raw_input, |ctx| {
                    if show_sidebar {
                        gui_actions = eye_control_panel(
                            ctx,
                            &mut state.left,
                            &mut state.right,
                            &mut state.link_shape,
                            &mut state.link_iris,
                            &mut state.link_eyebrow,
                            &mut state.link_eyelash,
                            &mut state.auto_blink,
                            &mut state.follow_mouse,
                            &mut state.show_highlight,
                            &mut state.show_eyebrow,
                            &mut state.show_eyelash,
                            &mut state.focus_distance,
                        );
                    }
                });

                // Handle GUI actions
                if gui_actions.export_requested {
                    let config = EyeConfig::from_state(
                        &state.left,
                        &state.right,
                        &state.link_shape,
                        &state.link_iris,
                        &state.link_eyebrow,
                        &state.link_eyelash,
                        state.auto_blink,
                        state.follow_mouse,
                        state.show_highlight,
                        state.show_eyebrow,
                        state.show_eyelash,
                        state.focus_distance,
                    );
                    if let Ok(json) = config.to_json() {
                        let file = rfd::FileDialog::new()
                            .set_title("Export Eye Config")
                            .add_filter("JSON", &["json"])
                            .set_file_name("eye_config.json")
                            .save_file();
                        if let Some(path) = file {
                            if let Err(e) = std::fs::write(&path, &json) {
                                eprintln!("Failed to write config: {e}");
                            }
                        }
                    }
                }

                if gui_actions.import_requested {
                    let file = rfd::FileDialog::new()
                        .set_title("Import Eye Config")
                        .add_filter("JSON", &["json"])
                        .pick_file();
                    if let Some(path) = file {
                        match std::fs::read_to_string(&path) {
                            Ok(json) => match EyeConfig::from_json(&json) {
                                Ok(config) => {
                                    config.apply_to_state(
                                        &mut state.left,
                                        &mut state.right,
                                        &mut state.link_shape,
                                        &mut state.link_iris,
                                        &mut state.link_eyebrow,
                                        &mut state.link_eyelash,
                                        &mut state.auto_blink,
                                        &mut state.follow_mouse,
                                        &mut state.show_highlight,
                                        &mut state.show_eyebrow,
                                        &mut state.show_eyelash,
                                        &mut state.focus_distance,
                                    );
                                }
                                Err(e) => eprintln!("Invalid config JSON: {e}"),
                            },
                            Err(e) => eprintln!("Failed to read config file: {e}"),
                        }
                    }
                }

                state
                    .egui_state
                    .handle_platform_output(&state.window, full_output.platform_output);

                let paint_jobs = state
                    .egui_ctx
                    .tessellate(full_output.shapes, full_output.pixels_per_point);
                let has_egui_content = !paint_jobs.is_empty();

                // Update egui textures only when there is content to render
                if has_egui_content {
                    for (id, delta) in &full_output.textures_delta.set {
                        state
                            .egui_renderer
                            .update_texture(&state.device, &state.queue, *id, delta);
                    }
                }

                let screen_descriptor = egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [state.surface_config.width, state.surface_config.height],
                    pixels_per_point: state.window.scale_factor() as f32,
                };

                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder =
                    state
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("eye_encoder"),
                        });

                // Update egui buffers only when there is content to render
                if has_egui_content {
                    state.egui_renderer.update_buffers(
                        &state.device,
                        &state.queue,
                        &mut encoder,
                        &paint_jobs,
                        &screen_descriptor,
                    );
                }

                // Render eye + egui overlay in same pass
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("eye_render_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    // Build paired uniforms with visibility overrides
                    let mut left_u = state.left.uniforms;
                    let mut right_u = state.right.uniforms;

                    if !state.show_highlight {
                        left_u.highlight_intensity = 0.0;
                        right_u.highlight_intensity = 0.0;
                    }
                    if !state.show_eyebrow {
                        left_u.eyebrow_base_y = 100.0;
                        right_u.eyebrow_base_y = 100.0;
                    }
                    if !state.show_eyelash {
                        left_u.eyelash_thickness = 0.0;
                        right_u.eyelash_thickness = 0.0;
                    }

                    let pair = EyePairUniforms {
                        left: left_u,
                        right: right_u,
                    };
                    state.queue.write_buffer(
                        state.renderer.uniform_buffer(),
                        0,
                        bytemuck::bytes_of(&pair),
                    );
                    pass.set_pipeline(state.renderer.pipeline());
                    pass.set_bind_group(0, state.renderer.bind_group(), &[]);
                    pass.draw(0..3, 0..1);

                    // Draw egui overlay (skip when nothing to render)
                    if has_egui_content {
                        state.egui_renderer.render(
                            &mut pass.forget_lifetime(),
                            &paint_jobs,
                            &screen_descriptor,
                        );
                    }
                }

                // Free egui textures
                for id in &full_output.textures_delta.free {
                    state.egui_renderer.free_texture(id);
                }

                state.queue.submit(std::iter::once(encoder.finish()));
                output.present();

                // Only request next frame when animation is running
                if state.auto_blink {
                    state.window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    env_logger::init();

    let config_path = std::env::args().nth(1).or_else(|| Some("eye_config.json".to_string()));

    let event_loop = EventLoop::new().unwrap();
    let mut app = App {
        state: None,
        config_path,
    };
    event_loop.run_app(&mut app).unwrap();
}
