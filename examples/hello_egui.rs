use egui::Context;
use wgpu_sandbox2::app;

struct HelloWorld {}

impl app::AppInstance for HelloWorld {
    fn create(_gpu: &wgpu_sandbox2::gpu::Gpu) -> Self {
        HelloWorld {}
    }

    fn run_egui(&self, ctx: &Context) {
        egui::Window::new("Hello").show(ctx, |ui| ui.label("this is a egui window"));
    }
}

fn main() {
    app::AppBuilder::new()
        .with_name("Hello World")
        .with_dimension(640, 480)
        .build()
        .run::<HelloWorld>();
}
