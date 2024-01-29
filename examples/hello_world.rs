use wgpu_sandbox2::app;

struct HelloWorld {}

impl app::AppInstance for HelloWorld {
    fn create(_gpu: &wgpu_sandbox2::gpu::Gpu) -> Self {
        HelloWorld {}
    }
}

fn main() {
    app::AppBuilder::new()
        .with_name("Hello World")
        .with_dimension(640, 480)
        .build()
        .run::<HelloWorld>();
}
