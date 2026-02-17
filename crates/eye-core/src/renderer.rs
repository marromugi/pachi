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
    pub _pad0: f32,                  // offset 12

    // -- Iris -- (48 bytes, offset 16)
    pub iris_offset: [f32; 2],       // offset 16  | vec2f (gaze drives this)
    pub iris_radius: f32,            // offset 24
    pub iris_noise_scale: f32,       // offset 28
    pub iris_color_inner: [f32; 3],  // offset 32  | vec3f
    pub _pad1: f32,                  // offset 44
    pub iris_color_outer: [f32; 3],  // offset 48  | vec3f
    pub _pad2: f32,                  // offset 60

    // -- Pupil -- (16 bytes, offset 64)
    pub pupil_color: [f32; 3],       // offset 64  | vec3f
    pub pupil_radius: f32,           // offset 76

    // -- Highlight -- (16 bytes, offset 80)
    pub highlight_offset: [f32; 2],  // offset 80  | vec2f
    pub highlight_radius: f32,       // offset 88
    pub highlight_intensity: f32,    // offset 92

    // -- Global -- (32 bytes, offset 96)
    pub bg_color: [f32; 3],          // offset 96  | vec3f
    pub eye_separation: f32,         // offset 108
    pub aspect_ratio: f32,           // offset 112
    pub time: f32,                   // offset 116
    pub eyelid_close: f32,           // offset 120 | 0.0 = open, 1.0 = closed
    pub show_iris_pupil: f32,        // offset 124 | 1.0 = show, 0.0 = hide
}
// Total: 128 bytes (= 16 * 8)

impl Default for EyeUniforms {
    fn default() -> Self {
        Self {
            // Sclera
            sclera_color: [0.95, 0.95, 0.95],
            _pad0: 0.0,

            // Iris
            iris_offset: [0.0, 0.0],
            iris_radius: 0.14,
            iris_noise_scale: 8.0,
            iris_color_inner: [0.35, 0.20, 0.08],
            _pad1: 0.0,
            iris_color_outer: [0.12, 0.07, 0.03],
            _pad2: 0.0,

            // Pupil
            pupil_color: [0.02, 0.02, 0.02],
            pupil_radius: 0.055,

            // Highlight
            highlight_offset: [-0.04, 0.06],
            highlight_radius: 0.03,
            highlight_intensity: 0.9,

            // Global
            bg_color: [0.0, 0.0, 0.0],
            eye_separation: 0.55,
            aspect_ratio: 16.0 / 9.0,
            time: 0.0,
            eyelid_close: 0.2,
            show_iris_pupil: 1.0,
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
