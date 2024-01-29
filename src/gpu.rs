use log::{error, info};

#[derive(Debug, Clone)]
pub struct GpuBuilder {
    pub(crate) backends: wgpu::Backends,
    pub(crate) device_type: wgpu::DeviceType,
    pub(crate) present_mode: wgpu::PresentMode,
    pub(crate) features: wgpu::Features,
    pub(crate) limits: wgpu::Limits,
}

impl GpuBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_backend(mut self, b: wgpu::Backend) -> Self {
        self.backends.insert(wgpu::Backends::from(b));
        self
    }

    pub fn with_device(mut self, d: wgpu::DeviceType) -> Self {
        self.device_type = d;
        self
    }

    pub fn with_feature(mut self, f: wgpu::Features) -> Self {
        self.features |= f;
        self
    }

    pub fn with_limits(mut self, l: wgpu::Limits) -> Self {
        self.limits = l;
        self
    }

    pub fn with_present_mode(mut self, p: wgpu::PresentMode) -> Self {
        self.present_mode = p;
        self
    }

    fn select_adapter(
        &self,
        instance: &wgpu::Instance,
        surface: &wgpu::Surface,
    ) -> Option<wgpu::Adapter> {
        let adapters = instance.enumerate_adapters(self.backends);
        for a in adapters {
            let info = a.get_info();
            if info.device_type == self.device_type
                && self.backends.contains(wgpu::Backends::from(info.backend))
                && a.is_surface_supported(&surface)
            {
                return Some(a);
            }
        }
        error!(
            target: "gpu_build",
            "Any device with these requirements :\n\tBACKENDS: {0:?}, \n\tDEVICE TYPE: {1:?} has been found",
            self.backends, self.device_type
        );
        None
    }

    pub async fn build(&self, window: &winit::window::Window) -> Gpu {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: self.backends,
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::debugging(),
            ..Default::default()
        });
        let surface = unsafe { instance.create_surface(window).unwrap() };
        let adapter = self.select_adapter(&instance, &surface).unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: self.features,
                    limits: self.limits.clone(),
                    label: Some("default device"),
                },
                None,
            )
            .await
            .unwrap();

        info!("running on : {}", adapter.get_info().name);

        let window_size = window.inner_size();

        let surface_caps = surface.get_capabilities(&adapter);
        let surace_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            format: surace_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: self.present_mode,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        Gpu {
            device,
            queue,
            surface,
            surface_config,
        }
    }
}

impl Default for GpuBuilder {
    fn default() -> Self {
        Self {
            backends: wgpu::Backends::PRIMARY,
            device_type: wgpu::DeviceType::DiscreteGpu,
            present_mode: wgpu::PresentMode::Fifo,
            features: wgpu::Features::empty(),
            limits: wgpu::Limits::default(),
        }
    }
}

#[derive(Debug)]
pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    pub(crate) surface_config: wgpu::SurfaceConfiguration,
}

impl Gpu {
    pub fn resize_surface(&mut self, new_size: (u32, u32)) {
        self.surface_config.width = new_size.0;
        self.surface_config.height = new_size.1;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn get_surface_texture_format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }
}
