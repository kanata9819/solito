use anyhow::Result;

use wgpu::{Adapter, Device, Instance, Queue, Surface};

pub(super) struct GpuContext {
    pub(super) instance: Instance,
    pub(super) adapter: Adapter,
    pub(super) device: Device,
    pub(super) queue: Queue,
}

impl GpuContext {
    pub(super) fn create_instance() -> Instance {
        // Avoid initializing Vulkan as well as DXGI on Windows. Keep wgpu's override.
        let backends = if cfg!(target_os = "windows") {
            wgpu::Backends::DX12
        } else {
            wgpu::Backends::PRIMARY
        };
        Instance::new(wgpu::InstanceDescriptor {
            backends: backends.with_env(),
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        })
    }

    pub(super) async fn new(instance: Instance, surface: &Surface<'_>) -> Result<Self> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await?;

        let (device, queue): (Device, Queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }
}
