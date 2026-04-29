//! GPU-accelerated graph renderer for Forgedsidian.
//!
//! This crate provides a `GraphRenderer` that uses wgpu to render thousands
//! of nodes and edges via GPU instancing. It replaces the SVG-based renderer
//! for performance at scale (1000+ nodes).
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐
//! │   Leptos UI      │  (forge-ui)
//! │  <canvas> elem   │
//! └──────┬──────────┘
//!        │ HtmlCanvasElement
//! ┌──────▼──────────┐
//! │  GraphRenderer   │  (this crate)
//! │  ├─ GpuContext    │  wgpu Device/Queue/Surface
//! │  ├─ GraphPipeline │  shaders, buffers, draw calls
//! │  └─ Camera2D      │  zoom, pan, projection
//! └──────┬──────────┘
//!        │ wgpu API
//! ┌──────▼──────────┐
//! │  GPU Backend     │  Vulkan / DX12 / Metal / WebGPU / WebGL2
//! └─────────────────┘
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! // 1. Initialize
//! let renderer = GraphRenderer::from_canvas(canvas_element, width, height).await?;
//!
//! // 2. Update data (on graph change or physics tick)
//! renderer.set_nodes(&node_instances);
//! renderer.set_edges(&edge_instances);
//!
//! // 3. Render each frame
//! renderer.render();
//!
//! // 4. Handle interaction
//! renderer.camera_mut().zoom_at(delta, cursor_pos);
//! renderer.camera_mut().pan(drag_delta);
//! ```

pub mod camera;
pub mod gpu;
pub mod graph_pipeline;
pub mod types;

pub use camera::Camera2D;
pub use gpu::{GpuContext, GpuInitError};
pub use graph_pipeline::GraphPipeline;
pub use types::{CameraUniform, EdgeInstance, NodeInstance};

// Re-export glam::Vec2 so downstream crates (forge-ui) don't need a direct glam dependency.
pub use glam::Vec2;

/// High-level graph renderer combining GPU context, pipeline, and camera.
///
/// This is the main entry point for forge-ui integration.
/// It owns all GPU resources and provides a simple API for
/// updating data and rendering frames.
pub struct GraphRenderer {
    pub gpu: GpuContext,
    pub pipeline: GraphPipeline,
    pub camera: Camera2D,
}

impl GraphRenderer {
    /// Create a new renderer attached to an HTML canvas element (WASM).
    #[cfg(target_arch = "wasm32")]
    pub async fn from_canvas(
        canvas: web_sys::HtmlCanvasElement,
        width: u32,
        height: u32,
    ) -> Result<Self, GpuInitError> {
        let gpu = GpuContext::new(wgpu::SurfaceTarget::Canvas(canvas), width, height).await?;
        let pipeline = GraphPipeline::new(&gpu);
        let camera = Camera2D::new(width as f32, height as f32);
        Ok(Self {
            gpu,
            pipeline,
            camera,
        })
    }

    /// Create a new renderer from a native window (desktop testing).
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn from_window(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Result<Self, GpuInitError> {
        let gpu = GpuContext::new(window, width, height).await?;
        let pipeline = GraphPipeline::new(&gpu);
        let camera = Camera2D::new(width as f32, height as f32);
        Ok(Self {
            gpu,
            pipeline,
            camera,
        })
    }

    /// Resize the rendering surface and camera viewport.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
        self.camera.resize(width as f32, height as f32);
    }

    /// Update node positions/colors on the GPU.
    pub fn set_nodes(&mut self, nodes: &[NodeInstance]) {
        self.pipeline.update_nodes(&self.gpu, nodes);
    }

    /// Update edge data on the GPU.
    pub fn set_edges(&mut self, edges: &[EdgeInstance]) {
        self.pipeline.update_edges(&self.gpu, edges);
    }

    /// Mutable access to the camera for zoom/pan.
    pub fn camera_mut(&mut self) -> &mut Camera2D {
        &mut self.camera
    }

    /// Render one frame.
    ///
    /// Updates the camera uniform, acquires a surface texture, and
    /// draws edges + nodes. Returns `Err` if the surface is lost
    /// (caller should recreate or resize).
    pub fn render(&self) -> Result<(), wgpu::SurfaceError> {
        let output = self.gpu.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Upload camera matrix.
        let cam_uniform = self.camera.uniform();
        self.pipeline.update_camera(&self.gpu, &cam_uniform);

        // Dark background matching Forgedsidian theme.
        let clear_color = wgpu::Color {
            r: 0.059, // #0F1923 (--trl-abyss)
            g: 0.098,
            b: 0.137,
            a: 1.0,
        };

        self.pipeline.render(&self.gpu, &view, clear_color);
        output.present();

        Ok(())
    }
}
