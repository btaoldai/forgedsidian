//! GPU-friendly flat data structures for graph rendering.
//!
//! These types are `#[repr(C)]` + `bytemuck::Pod` so they can be copied
//! directly into wgpu vertex/instance buffers with zero conversion cost.
//! The GPU sees them as raw byte arrays — no pointers, no indirection.

use bytemuck::{Pod, Zeroable};

// ───────────────────────── Node Instance ─────────────────────────

/// Per-node data sent to the GPU as an instance buffer.
///
/// Each node is rendered as a textured quad that the fragment shader
/// clips into a circle using a signed distance function (SDF).
///
/// # Layout
///
/// Matches the WGSL struct `NodeInstance` in `node.wgsl`.
/// Fields are `f32` for GPU compatibility (GPU prefers f32 over f64).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct NodeInstance {
    /// Center position in graph-space coordinates.
    pub position: [f32; 2],
    /// Circle radius in graph-space units.
    pub radius: f32,
    /// RGBA color (0.0–1.0 per channel).
    pub color: [f32; 4],
    /// Unique numeric ID for GPU picking (encoded as color in pick pass).
    pub pick_id: u32,
}

impl NodeInstance {
    /// Byte size of one instance (for wgpu buffer stride).
    pub const SIZE: u64 = std::mem::size_of::<Self>() as u64;

    /// wgpu vertex buffer layout describing how the GPU reads per-instance data.
    ///
    /// Slot 1 (slot 0 is the base quad vertices).
    pub fn instance_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: Self::SIZE,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // @location(2) position: vec2<f32>
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 2,
                },
                // @location(3) radius: f32
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 8,
                    shader_location: 3,
                },
                // @location(4) color: vec4<f32>
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 12,
                    shader_location: 4,
                },
                // @location(5) pick_id: u32
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Uint32,
                    offset: 28,
                    shader_location: 5,
                },
            ],
        }
    }
}

// ───────────────────────── Edge Instance ─────────────────────────

/// Per-edge data sent to the GPU as an instance buffer.
///
/// Each edge is rendered as an oriented quad (rectangle) stretched
/// between two node centers. The vertex shader computes the quad
/// corners from `start`, `end`, and `thickness`.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct EdgeInstance {
    /// Start position (source node center) in graph-space.
    pub start: [f32; 2],
    /// End position (target node center) in graph-space.
    pub end: [f32; 2],
    /// Line thickness in graph-space units.
    pub thickness: f32,
    /// RGBA color (0.0–1.0 per channel).
    pub color: [f32; 4],
    /// Padding to align to 16 bytes (GPU likes aligned data).
    pub _pad: [f32; 1],
}

impl EdgeInstance {
    pub const SIZE: u64 = std::mem::size_of::<Self>() as u64;

    pub fn instance_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: Self::SIZE,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // @location(2) start: vec2<f32>
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 2,
                },
                // @location(3) end: vec2<f32>
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 8,
                    shader_location: 3,
                },
                // @location(4) thickness: f32
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 16,
                    shader_location: 4,
                },
                // @location(5) color: vec4<f32>
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 20,
                    shader_location: 5,
                },
            ],
        }
    }
}

// ───────────────────────── Base Quad Vertex ─────────────────────────

/// Vertex of the unit quad [-1, 1] used as base geometry for instancing.
///
/// Each node/edge instance transforms this quad via the vertex shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct QuadVertex {
    /// Position in local quad space (-1 to 1).
    pub position: [f32; 2],
}

impl QuadVertex {
    pub const SIZE: u64 = std::mem::size_of::<Self>() as u64;

    pub fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: Self::SIZE,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // @location(0) position: vec2<f32>
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
            ],
        }
    }
}

/// Unit quad vertices: two triangles forming a [-1, 1] square.
/// The fragment shader uses this to compute SDF (distance from center).
pub const QUAD_VERTICES: &[QuadVertex] = &[
    QuadVertex {
        position: [-1.0, -1.0],
    },
    QuadVertex {
        position: [1.0, -1.0],
    },
    QuadVertex {
        position: [1.0, 1.0],
    },
    QuadVertex {
        position: [-1.0, -1.0],
    },
    QuadVertex {
        position: [1.0, 1.0],
    },
    QuadVertex {
        position: [-1.0, 1.0],
    },
];

// ───────────────────────── Camera Uniform ─────────────────────────

/// Camera data sent to the GPU as a uniform buffer.
///
/// Contains the orthographic projection * view matrix.
/// Updated every frame when the user pans/zooms.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CameraUniform {
    /// Combined projection * view matrix (column-major, as wgpu expects).
    pub view_proj: [[f32; 4]; 4],
}
