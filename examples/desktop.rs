use std::sync::Arc;
use std::time::Instant;

use eye::gui::eye_control_panel;
use eye::{BlinkAnimation, EyeRenderer, EyeShape, EyebrowShape, EyeUniforms};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

struct App {
    state: Option<AppState>,
}

struct AppState {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    renderer: EyeRenderer,
    uniforms: EyeUniforms,
    eye_shape: EyeShape,
    eyebrow_shape: EyebrowShape,
    blink_animation: BlinkAnimation,
    auto_blink: bool,
    follow_mouse: bool,
    show_highlight: bool,
    show_eyebrow: bool,
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

        let state = pollster::block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                ..Default::default()
            });

            let surface = instance.create_surface(window.clone()).unwrap();

            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
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
            let uniforms = EyeUniforms::default();
            let eye_shape = EyeShape::default();

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
                uniforms,
                eye_shape,
                eyebrow_shape: EyebrowShape::default(),
                blink_animation: BlinkAnimation::sample(),
                auto_blink: true,
                follow_mouse: true,
                show_highlight: true,
                show_eyebrow: true,
                focus_distance: 20.0,
                mouse_position: None,
                start_time: Instant::now(),
                egui_ctx,
                egui_state,
                egui_renderer,
            }
        });

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

        // Pass events to egui first
        let egui_response = state.egui_state.on_window_event(&state.window, &event);
        if egui_response.consumed {
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

                // Update dynamic uniforms
                state.uniforms.aspect_ratio =
                    state.surface_config.width as f32 / state.surface_config.height as f32;
                state.uniforms.time = state.start_time.elapsed().as_secs_f32();

                if state.auto_blink {
                    let eyelid_now =
                        state.blink_animation.evaluate(state.uniforms.time);

                    // Squash & stretch driven by eyelid velocity
                    let dt = 1.0 / 60.0_f32;
                    let eyelid_prev =
                        state.blink_animation.evaluate(state.uniforms.time - dt);
                    let velocity = (eyelid_now - eyelid_prev) / dt;
                    const SQUASH_STRENGTH: f32 = 0.08;
                    const MAX_SQUASH: f32 = 0.045;
                    state.uniforms.squash_stretch =
                        (velocity * SQUASH_STRENGTH).clamp(-MAX_SQUASH, MAX_SQUASH);

                    state.uniforms.eyelid_close = eyelid_now;
                } else {
                    state.uniforms.squash_stretch = 0.0;
                }

                // Mouse follow → look_x / look_y
                if state.follow_mouse {
                    if let Some(pos) = state.mouse_position {
                        let cx = state.surface_config.width as f64 / 2.0;
                        let cy = state.surface_config.height as f64 / 2.0;
                        state.uniforms.look_x =
                            ((pos.x - cx) / cx).clamp(-1.0, 1.0) as f32;
                        state.uniforms.look_y =
                            -((pos.y - cy) / cy).clamp(-1.0, 1.0) as f32;
                    }
                }

                // Focus distance → convergence offset
                let half_ipd = state.uniforms.eye_separation * 0.5;
                state.uniforms.convergence = (half_ipd / state.focus_distance * 0.08)
                    .clamp(0.0, state.uniforms.iris_follow * 0.8);

                // Sync eye shape into uniforms
                state.uniforms.outline_open = state.eye_shape.open.to_uniform_array();
                state.uniforms.outline_closed = state.eye_shape.closed.to_uniform_array();

                // Sync eyebrow shape into uniforms
                state.uniforms.eyebrow_color = state.eyebrow_shape.color;
                state.uniforms.eyebrow_base_y = state.eyebrow_shape.base_y;
                state.uniforms.eyebrow_follow = state.eyebrow_shape.follow;
                state.uniforms.eyebrow_outline = state.eyebrow_shape.outline.to_uniform_array();

                // --- egui frame ---
                let raw_input = state.egui_state.take_egui_input(&state.window);
                let full_output = state.egui_ctx.run(raw_input, |ctx| {
                    eye_control_panel(ctx, &mut state.uniforms, &mut state.eye_shape, &mut state.eyebrow_shape, &mut state.auto_blink, &mut state.follow_mouse, &mut state.show_highlight, &mut state.show_eyebrow, &mut state.focus_distance);
                });

                state
                    .egui_state
                    .handle_platform_output(&state.window, full_output.platform_output);

                let paint_jobs = state
                    .egui_ctx
                    .tessellate(full_output.shapes, full_output.pixels_per_point);

                // Update egui textures
                for (id, delta) in &full_output.textures_delta.set {
                    state
                        .egui_renderer
                        .update_texture(&state.device, &state.queue, *id, delta);
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

                // Update egui buffers
                state.egui_renderer.update_buffers(
                    &state.device,
                    &state.queue,
                    &mut encoder,
                    &paint_jobs,
                    &screen_descriptor,
                );

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

                    // Draw eye
                    let saved_highlight = state.uniforms.highlight_intensity;
                    let saved_eyebrow_base_y = state.uniforms.eyebrow_base_y;
                    if !state.show_highlight {
                        state.uniforms.highlight_intensity = 0.0;
                    }
                    if !state.show_eyebrow {
                        state.uniforms.eyebrow_base_y = 100.0;
                    }
                    state.queue.write_buffer(
                        state.renderer.uniform_buffer(),
                        0,
                        bytemuck::bytes_of(&state.uniforms),
                    );
                    state.uniforms.highlight_intensity = saved_highlight;
                    state.uniforms.eyebrow_base_y = saved_eyebrow_base_y;
                    pass.set_pipeline(state.renderer.pipeline());
                    pass.set_bind_group(0, state.renderer.bind_group(), &[]);
                    pass.draw(0..3, 0..1);

                    // Draw egui overlay
                    state.egui_renderer.render(
                        &mut pass.forget_lifetime(),
                        &paint_jobs,
                        &screen_descriptor,
                    );
                }

                // Free egui textures
                for id in &full_output.textures_delta.free {
                    state.egui_renderer.free_texture(id);
                }

                state.queue.submit(std::iter::once(encoder.finish()));
                output.present();

                state.window.request_redraw();
            }
            _ => {}
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App { state: None };
    event_loop.run_app(&mut app).unwrap();
}
