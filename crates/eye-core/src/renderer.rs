use crate::outline::BezierOutline;

/// GPU uniform structure for a single canonical eye.
/// The shader mirrors the X coordinate to render two eyes.
///
/// WGSL alignment rules:
///   vec2f = align 8, size 8
///   vec3f = align 16, size 12 (pad to 16 with trailing f32)
///   f32   = align 4, size 4
/// Total struct size must be a multiple of 16.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EyeUniforms {
    // -- Sclera (white part) -- (16 bytes, offset 0)
    pub sclera_color: [f32; 3],      // offset 0   | vec3f
    pub squash_stretch: f32,         // offset 12  | >0 = squash, <0 = stretch

    // -- Highlight -- (16 bytes, offset 16)
    pub highlight_offset: [f32; 2],  // offset 16  | vec2f
    pub highlight_radius: f32,       // offset 24
    pub highlight_intensity: f32,    // offset 28

    // -- Global -- (32 bytes, offset 32)
    pub bg_color: [f32; 3],          // offset 32  | vec3f
    pub eye_separation: f32,         // offset 44
    pub aspect_ratio: f32,           // offset 48
    pub time: f32,                   // offset 52
    pub eyelid_close: f32,           // offset 56  | 0.0 = open, 1.0 = closed
    pub look_x: f32,                 // offset 60  | [-1, 1] horizontal gaze

    // -- Perspective -- (16 bytes, offset 64)
    pub look_y: f32,                 // offset 64  | [-1, 1] vertical gaze
    pub max_angle: f32,              // offset 68  | max rotation angle (radians)
    pub eye_angle: f32,              // offset 72  | eye angular half-separation (radians)
    pub _pad_perspective: f32,       // offset 76

    // -- Iris -- (32 bytes, offset 80)
    pub iris_color: [f32; 3],        // offset 80  | vec3f - iris color
    pub iris_radius: f32,            // offset 92  | iris circle radius
    pub iris_follow: f32,            // offset 96  | gaze follow scale
    pub _pad_iris: [f32; 3],         // offset 100 | padding to 16-byte boundary

    // -- Bezier outline open -- (128 bytes, offset 112)
    // 4 segments x 2 vec4f each. Each vec4f packs 2 vec2f control points.
    // seg[i*2]   = (P0.xy, P1.xy) = (anchor, anchor+handle_out)
    // seg[i*2+1] = (P2.xy, P3.xy) = (next_anchor+handle_in, next_anchor)
    pub outline_open: [[f32; 4]; 8],

    // -- Bezier outline closed -- (128 bytes, offset 240)
    pub outline_closed: [[f32; 4]; 8],

    // -- Eyebrow -- (160 bytes, offset 368)
    pub eyebrow_color: [f32; 3],         // offset 368 | vec3f
    pub eyebrow_base_y: f32,             // offset 380 | base Y position above eye
    pub eyebrow_follow: f32,             // offset 384 | eyelid follow rate
    pub _pad_eyebrow: [f32; 3],          // offset 388 | padding to 16-byte boundary
    pub eyebrow_outline: [[f32; 4]; 8],  // offset 400 | Bezier control points
}
// Total: 528 bytes (= 16 * 33)

const _: () = assert!(std::mem::size_of::<EyeUniforms>() == 528);

impl Default for EyeUniforms {
    fn default() -> Self {
        Self {
            // Sclera
            sclera_color: [0.95, 0.95, 0.95],
            squash_stretch: 0.0,

            // Highlight
            highlight_offset: [-0.04, 0.06],
            highlight_radius: 0.03,
            highlight_intensity: 0.9,

            // Global
            bg_color: [0.045, 0.097, 0.199],
            eye_separation: 1.20,
            aspect_ratio: 16.0 / 9.0,
            time: 0.0,
            eyelid_close: 0.2,
            look_x: 0.0,

            // Perspective
            look_y: 0.0,
            max_angle: 0.5,
            eye_angle: 0.8,
            _pad_perspective: 0.0,

            // Iris
            iris_color: [0.009, 0.009, 0.035],
            iris_radius: 0.2,
            iris_follow: 0.14,
            _pad_iris: [0.0, 0.0, 0.0],

            // Bezier outline
            outline_open: BezierOutline::ellipse(0.28, 0.35).to_uniform_array(),
            outline_closed: BezierOutline::closed_slit_asymmetric(0.20, -0.20).to_uniform_array(),

            // Eyebrow
            eyebrow_color: [0.009, 0.009, 0.035],
            eyebrow_base_y: 0.48,
            eyebrow_follow: 0.15,
            _pad_eyebrow: [0.0, 0.0, 0.0],
            eyebrow_outline: BezierOutline::eyebrow_arc(0.30, 0.04).to_uniform_array(),
        }
    }
}

pub struct EyeRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl EyeRenderer {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("eye_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/eye.wgsl").into()),
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("eye_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("eye_uniform_buffer"),
            size: std::mem::size_of::<EyeUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("eye_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("eye_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("eye_render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
        }
    }

    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn uniform_buffer(&self) -> &wgpu::Buffer {
        &self.uniform_buffer
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        queue: &wgpu::Queue,
        params: &EyeUniforms,
    ) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(params));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("eye_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
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

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
