//! wgpu initialization and GPU context management.
//!
//! `GpuContext` owns the wgpu `Device`, `Queue`, and `Surface`.
//! It handles surface configuration, resize, and provides the
//! foundation for all rendering pipelines.

use tracing;

/// Holds the core wgpu objects needed by all rendering pipelines.
///
/// Created once at startup, lives for the entire application lifetime.
/// Call `resize()` when the canvas/window dimensions change.
pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface_format: wgpu::TextureFormat,
}

impl GpuContext {
    /// Initialize wgpu and create a rendering surface.
    ///
    /// # WASM
    ///
    /// On WASM, `target` must be a `web_sys::HtmlCanvasElement`.
    /// The function is async because adapter/device request are async.
    ///
    /// # Native
    ///
    /// On native, `target` is typically a `winit::Window` reference.
    pub async fn new(
        target: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Result<Self, GpuInitError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance
            .create_surface(target)
            .map_err(|e| GpuInitError::Surface(e.to_string()))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|_| GpuInitError::NoAdapter)?;

        tracing::info!(
            backend = ?adapter.get_info().backend,
            name = adapter.get_info().name,
            "GPU adapter selected"
        );

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("forge-renderer"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::Performance,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| GpuInitError::Device(e.to_string()))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        tracing::info!(
            width,
            height,
            format = ?surface_format,
            "GPU surface configured"
        );

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            surface_format,
        })
    }

    /// Resize the rendering surface (call when canvas/window changes size).
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
        tracing::debug!(width, height, "GPU surface resized");
    }

    /// Current surface width in pixels.
    pub fn width(&self) -> u32 {
        self.surface_config.width
    }

    /// Current surface height in pixels.
    pub fn height(&self) -> u32 {
        self.surface_config.height
    }
}

/// Errors that can occur during GPU initialization.
#[derive(Debug, Clone)]
pub enum GpuInitError {
    /// Failed to create the wgpu surface.
    Surface(String),
    /// No compatible GPU adapter found.
    NoAdapter,
    /// Failed to create the GPU device.
    Device(String),
}

impl std::fmt::Display for GpuInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Surface(e) => write!(f, "GPU surface creation failed: {e}"),
            Self::NoAdapter => write!(f, "No compatible GPU adapter found"),
            Self::Device(e) => write!(f, "GPU device creation failed: {e}"),
        }
    }
}

impl std::error::Error for GpuInitError {}
