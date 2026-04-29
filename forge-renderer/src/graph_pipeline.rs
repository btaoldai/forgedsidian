//! Instanced rendering pipeline for graph nodes and edges.
//!
//! Uses wgpu to render thousands of circles (nodes) and lines (edges)
//! in a single draw call each, via GPU instancing. This is orders of
//! magnitude faster than SVG DOM manipulation for large graphs.

use crate::gpu::GpuContext;
use crate::types::{CameraUniform, EdgeInstance, NodeInstance, QuadVertex, QUAD_VERTICES};
use wgpu::util::DeviceExt;

/// Rendering pipeline for the graph view.
///
/// Owns the GPU buffers, shader modules, and render pipelines.
/// Call `update_nodes()` / `update_edges()` when the graph data changes,
/// then `render()` each frame.
pub struct GraphPipeline {
    // Shared
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    quad_vertex_buffer: wgpu::Buffer,

    // Node pipeline
    node_pipeline: wgpu::RenderPipeline,
    node_instance_buffer: wgpu::Buffer,
    node_count: u32,
    node_buffer_capacity: u64,

    // Edge pipeline
    edge_pipeline: wgpu::RenderPipeline,
    edge_instance_buffer: wgpu::Buffer,
    edge_count: u32,
    edge_buffer_capacity: u64,
}

impl GraphPipeline {
    /// Create the rendering pipeline.
    ///
    /// Compiles shaders, creates buffers, and sets up the render pipelines.
    /// Call once at initialization (after `GpuContext::new()`).
    pub fn new(gpu: &GpuContext) -> Self {
        let device = &gpu.device;

        // ── Camera uniform buffer + bind group ──
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera_uniform"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bg"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // ── Shared quad vertex buffer ──
        let quad_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad_vb"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // ── Pipeline layout (shared) ──
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("graph_pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        // ── Node shader + pipeline ──
        let node_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("node_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/node.wgsl").into()),
        });

        let node_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("node_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &node_shader,
                entry_point: Some("vs_main"),
                buffers: &[QuadVertex::vertex_layout(), NodeInstance::instance_layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &node_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        // ── Edge shader + pipeline ──
        let edge_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("edge_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/edge.wgsl").into()),
        });

        let edge_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("edge_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &edge_shader,
                entry_point: Some("vs_main"),
                buffers: &[QuadVertex::vertex_layout(), EdgeInstance::instance_layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &edge_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        // ── Initial empty instance buffers (will grow on first update) ──
        let initial_capacity = 1024u64;
        let node_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("node_instances"),
            size: initial_capacity * NodeInstance::SIZE,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let edge_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("edge_instances"),
            size: initial_capacity * EdgeInstance::SIZE,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            camera_buffer,
            camera_bind_group,
            quad_vertex_buffer,
            node_pipeline,
            node_instance_buffer,
            node_count: 0,
            node_buffer_capacity: initial_capacity,
            edge_pipeline,
            edge_instance_buffer,
            edge_count: 0,
            edge_buffer_capacity: initial_capacity,
        }
    }

    /// Upload camera matrix to the GPU.
    ///
    /// Call every frame (or when camera changes).
    pub fn update_camera(&self, gpu: &GpuContext, uniform: &CameraUniform) {
        gpu.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(uniform));
    }

    /// Upload node instance data to the GPU.
    ///
    /// Automatically grows the buffer if the current capacity is too small.
    /// Call when the graph data changes (new snapshot or physics tick).
    pub fn update_nodes(&mut self, gpu: &GpuContext, nodes: &[NodeInstance]) {
        self.node_count = nodes.len() as u32;
        if nodes.is_empty() {
            return;
        }

        let required = nodes.len() as u64;
        if required > self.node_buffer_capacity {
            // Grow with 50% headroom to avoid frequent reallocations.
            let new_capacity = (required * 3 / 2).max(256);
            self.node_instance_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("node_instances"),
                size: new_capacity * NodeInstance::SIZE,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.node_buffer_capacity = new_capacity;
            tracing::debug!(new_capacity, "Node instance buffer grown");
        }

        gpu.queue
            .write_buffer(&self.node_instance_buffer, 0, bytemuck::cast_slice(nodes));
    }

    /// Upload edge instance data to the GPU.
    ///
    /// Same growth strategy as `update_nodes`.
    pub fn update_edges(&mut self, gpu: &GpuContext, edges: &[EdgeInstance]) {
        self.edge_count = edges.len() as u32;
        if edges.is_empty() {
            return;
        }

        let required = edges.len() as u64;
        if required > self.edge_buffer_capacity {
            let new_capacity = (required * 3 / 2).max(256);
            self.edge_instance_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("edge_instances"),
                size: new_capacity * EdgeInstance::SIZE,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.edge_buffer_capacity = new_capacity;
            tracing::debug!(new_capacity, "Edge instance buffer grown");
        }

        gpu.queue
            .write_buffer(&self.edge_instance_buffer, 0, bytemuck::cast_slice(edges));
    }

    /// Render the graph (edges first, then nodes on top).
    ///
    /// Call once per frame after `update_camera()`.
    /// `view` is the texture view of the current surface frame.
    /// `clear_color` is the background color.
    pub fn render(&self, gpu: &GpuContext, view: &wgpu::TextureView, clear_color: wgpu::Color) {
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("graph_render"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("graph_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Draw edges first (behind nodes).
            if self.edge_count > 0 {
                pass.set_pipeline(&self.edge_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
                pass.set_vertex_buffer(1, self.edge_instance_buffer.slice(..));
                pass.draw(0..6, 0..self.edge_count); // 6 vertices per quad
            }

            // Draw nodes on top.
            if self.node_count > 0 {
                pass.set_pipeline(&self.node_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
                pass.set_vertex_buffer(1, self.node_instance_buffer.slice(..));
                pass.draw(0..6, 0..self.node_count);
            }
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));
    }
}
