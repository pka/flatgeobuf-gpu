use flatgeobuf::*;
use pathfinder_canvas::{Canvas, CanvasFontContext, CanvasRenderingContext2D, Path2D};
use pathfinder_color::ColorF;
use pathfinder_geometry::vector::{vec2f, vec2i};
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::concurrent::rayon::RayonExecutor;
use pathfinder_renderer::concurrent::scene_proxy::SceneProxy;
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_renderer::options::BuildOptions;
use pathfinder_resources::embedded::EmbeddedResourceLoader;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;
use std::fs::File;
use std::io::BufReader;

struct PathDrawer<'a> {
    xfact: f32,
    yfact: f32,
    canvas: &'a mut CanvasRenderingContext2D,
    path: Path2D,
}

impl<'a> GeomReader for PathDrawer<'a> {
    fn pointxy(&mut self, x: f64, y: f64, idx: usize) {
        let x = 180.0 + x as f32;
        let y = 90.0 - y as f32;
        if idx == 0 {
            self.path.move_to(vec2f(x * self.xfact, y * self.yfact));
        } else {
            self.path.line_to(vec2f(x * self.xfact, y * self.yfact));
        }
    }
    fn ring_begin(&mut self, _size: usize, _idx: usize) {
        self.path = Path2D::new();
    }
    fn ring_end(&mut self, _idx: usize) {
        self.path.close_path();
        self.canvas.stroke_path(self.path.clone()); // Do we really need Copy/Clone?
    }
}

fn main() -> std::result::Result<(), std::io::Error> {
    // Set up SDL2.
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();

    // Make sure we have at least a GL 3.0 context. Pathfinder requires this.
    let gl_attributes = video.gl_attr();
    gl_attributes.set_context_profile(GLProfile::Core);
    gl_attributes.set_context_version(3, 3);

    // Open a window.
    let window_size = vec2i(2048, 1024);
    let window = video
        .window(
            "Minimal example",
            window_size.x() as u32,
            window_size.y() as u32,
        )
        .opengl()
        .build()
        .unwrap();

    // Create the GL context, and make it current.
    let gl_context = window.gl_create_context().unwrap();
    gl::load_with(|name| video.gl_get_proc_address(name) as *const _);
    window.gl_make_current(&gl_context).unwrap();

    // Create a Pathfinder renderer.
    let mut renderer = Renderer::new(
        GLDevice::new(GLVersion::GL3, 0),
        &EmbeddedResourceLoader::new(),
        DestFramebuffer::full_window(window_size),
        RendererOptions {
            background_color: Some(ColorF::white()),
        },
    );

    // Make a canvas. We're going to draw a house.
    let font_context = CanvasFontContext::from_system_source();
    let mut canvas = Canvas::new(window_size.to_f32()).get_context_2d(font_context);
    canvas.set_line_width(1.0);

    let mut file = BufReader::new(File::open(
        "/home/pi/code/gis/flatgeobuf/test/data/countries.fgb",
    )?);
    let hreader = HeaderReader::read(&mut file)?;
    let header = hreader.header();

    // let mut drawer = DebugReader {};
    let mut drawer = PathDrawer {
        xfact: window_size.x() as f32 / 360.0,
        yfact: window_size.y() as f32 / 180.0,
        canvas: &mut canvas,
        path: Path2D::new(),
    };
    let mut freader = FeatureReader::select_all(&mut file, &header)?;
    while let Ok(feature) = freader.next(&mut file) {
        let geometry = feature.geometry().unwrap();
        geometry.parse(&mut drawer, header.geometry_type());
    }

    // Render the canvas to screen.
    let scene = SceneProxy::from_scene(canvas.into_canvas().into_scene(), RayonExecutor);
    scene.build_and_render(&mut renderer, BuildOptions::default());
    window.gl_swap_window();

    // Wait for a keypress.
    let mut event_pump = sdl_context.event_pump().unwrap();
    loop {
        match event_pump.wait_event() {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => return Ok(()),
            _ => {}
        }
    }
}
