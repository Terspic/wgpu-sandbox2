use std::time::{Duration, Instant};

#[cfg(feature = "egui")]
use egui_wgpu::renderer::ScreenDescriptor;

use futures_lite::future::block_on;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

use crate::gpu::{Gpu, GpuBuilder};

#[cfg(feature = "egui")]
use crate::egui_renderer::EguiRenderer;

pub trait AppInstance {
    /// create an isntance of the app
    fn create(gpu: &Gpu) -> Self;

    /// handle window events
    fn events(&mut self, _event: &winit::event::WindowEvent) {}

    /// update the current of state
    fn update(&mut self, _gpu: &Gpu, _dt: Duration) {}

    /// render the current frame
    fn render(
        &self,
        gpu: &Gpu,
        frame_view: &wgpu::TextureView,
    ) -> Option<Vec<wgpu::CommandBuffer>> {
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });
        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    view: &frame_view,
                    resolve_target: None,
                })],
                ..Default::default()
            });
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));
        None
    }

    /// destroy the app
    fn destroy(&self) {}

    #[cfg(feature = "egui")]
    fn run_egui(&self, ctx: &egui::Context);
}

/// builder for the struct App
#[derive(Debug, Clone)]
pub struct AppBuilder {
    /// name of the application
    name: String,
    /// window dimension
    dim: (u32, u32),
    /// builder for the gpu absatraction
    gpu_builder: GpuBuilder,
    /// if true, init logging for wgpu
    init_subscriber: bool,
    /// set if the window of the app is resizable
    resizable: bool,
    /// enale exiting the app with the escape key
    esc: bool,
}

impl AppBuilder {
    /// build a builder with default options (wrapper to AppBuilder::default)
    pub fn new() -> Self {
        Self::default()
    }

    /// change the name of the app
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = String::from(name);
        self
    }

    /// change the default Gpu Builder
    /// use it to select a specific gpu
    pub fn with_gpu(mut self, gpu_builder: GpuBuilder) -> Self {
        self.gpu_builder = gpu_builder;
        self
    }

    /// change the window dimensions of the app
    pub fn with_dimension(mut self, widht: u32, height: u32) -> Self {
        self.dim = (widht, height);
        self
    }

    /// boolean to set if you want to enable wgpu logging
    pub fn with_init_subscriber(mut self, value: bool) -> Self {
        self.init_subscriber = value;
        self
    }

    /// boolean that set if the window should be resizable
    pub fn with_resizable(mut self, value: bool) -> Self {
        self.resizable = value;
        self
    }

    /// set if the app should exit when escape key is pressed
    pub fn with_esc(mut self, esc: bool) -> Self {
        self.esc = esc;
        self
    }

    /// build the app
    pub fn build(&self) -> App {
        if self.init_subscriber {
            env_logger::init();
        }

        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(self.dim.0, self.dim.1))
            .with_title(self.name.as_str())
            .with_resizable(self.resizable)
            .build(&event_loop)
            .unwrap();

        let gpu = block_on(self.gpu_builder.build(&window));

        #[cfg(feature = "egui")]
        let renderer = EguiRenderer::new(&gpu.device, gpu.surface_config.format, None, 1, &window);

        App {
            window,
            event_loop,
            gpu,
            esc: self.esc,

            #[cfg(feature = "egui")]
            egui_renderer: renderer,
        }
    }
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self {
            name: String::from("default app"),
            dim: (640, 360),
            gpu_builder: GpuBuilder::default(),
            init_subscriber: true,
            resizable: false,
            esc: true,
        }
    }
}

pub struct App {
    window: winit::window::Window,
    event_loop: EventLoop<()>,
    gpu: Gpu,
    esc: bool,

    #[cfg(feature = "egui")]
    egui_renderer: EguiRenderer,
}

impl App {
    pub fn run<T: AppInstance + 'static>(mut self) {
        // build app
        let mut instance = T::create(&self.gpu);

        let mut last_frame = Instant::now();

        self.event_loop
            .run(move |event, _, control_flow| match event {
                Event::WindowEvent { ref event, .. } => {
                    match event {
                        WindowEvent::CloseRequested => control_flow.set_exit(),
                        WindowEvent::KeyboardInput { input, .. } => match input {
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                state: ElementState::Pressed,
                                ..
                            } => {
                                if self.esc {
                                    control_flow.set_exit();
                                }
                            }
                            _ => (),
                        },
                        // resize the surface
                        WindowEvent::Resized(size) => {
                            self.gpu.resize_surface((size.width, size.height));
                        }
                        _ => (),
                    }

                    // send the event to the app
                    instance.events(event);

                    #[cfg(feature = "egui")]
                    self.egui_renderer.handle_input(event);
                }
                Event::MainEventsCleared => self.window.request_redraw(),
                Event::RedrawRequested(_) => {
                    // update the app
                    let now = Instant::now();
                    instance.update(&self.gpu, now - last_frame);
                    last_frame = now;

                    // render
                    match self.gpu.surface.get_current_texture() {
                        Ok(frame) => {
                            let frame_view = frame
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            let cmd_bufs =
                                instance.render(&self.gpu, &frame_view).unwrap_or(vec![]);

                            self.gpu.queue.submit(cmd_bufs.into_iter());

                            // draw egui
                            #[cfg(feature = "egui")]
                            {
                                let screen_desc = ScreenDescriptor {
                                    size_in_pixels: [
                                        self.gpu.surface_config.width,
                                        self.gpu.surface_config.height,
                                    ],
                                    pixels_per_point: self.window.scale_factor() as f32,
                                };

                                let mut egui_encoder = self.gpu.device.create_command_encoder(
                                    &wgpu::CommandEncoderDescriptor {
                                        label: Some("egui_command_encoder"),
                                    },
                                );

                                self.egui_renderer.draw(
                                    &self.gpu.device,
                                    &self.gpu.queue,
                                    &mut egui_encoder,
                                    &self.window,
                                    &frame_view,
                                    screen_desc,
                                    |ui| instance.run_egui(ui),
                                );
                                self.gpu
                                    .queue
                                    .submit(std::iter::once(egui_encoder.finish()));
                            }

                            frame.present();
                        }
                        Err(wgpu::SurfaceError::Outdated) => {
                            println!("Surface outdated, skip frame")
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
                Event::LoopDestroyed => {
                    instance.destroy();
                }
                _ => (),
            });
    }
}
