use flatgeobuf::*;
use pathfinder_canvas::{Canvas, CanvasFontContext, CanvasRenderingContext2D, Path2D};
use pathfinder_color::{rgbu, ColorF};
use pathfinder_content::fill::FillRule;
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
    xmin: f32,
    ymax: f32,
    canvas: &'a mut CanvasRenderingContext2D,
    path: Path2D,
}

impl<'a> GeomReader for PathDrawer<'a> {
    fn pointxy(&mut self, x: f64, y: f64, idx: usize) {
        // x,y are in degrees, y must be inverted
        let x = (x as f32 - self.xmin) * self.xfact;
        let y = (self.ymax - y as f32) * self.yfact;
        if idx == 0 {
            self.path.move_to(vec2f(x, y));
        } else {
            self.path.line_to(vec2f(x, y));
        }
    }
    fn ring_begin(&mut self, _size: usize, _idx: usize) {
        self.path = Path2D::new();
    }
    fn ring_end(&mut self, _idx: usize) {
        self.path.close_path();
        self.canvas.fill_path(self.path.clone(), FillRule::Winding);
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
            "FlatGeobuf Demo",
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
    canvas.set_fill_style(rgbu(132, 132, 132));

    let mut file = BufReader::new(File::open(
        "/home/pi/code/gis/flatgeobuf/test/data/osm/osm-buildings-ch.fgb",
        // "/home/pi/code/gis/flatgeobuf/test/data/countries.fgb",
    )?);
    let hreader = HeaderReader::read(&mut file)?;
    let header = hreader.header();

    let bbox = (8.522086, 47.363333, 8.553521, 47.376020);
    // let bbox = (-180.0, -90.0, 180.0, 90.0);
    let w = (bbox.2 - bbox.0) as f32;
    let h = (bbox.3 - bbox.1) as f32;

    // let mut drawer = DebugReader {};
    let mut drawer = PathDrawer {
        xfact: window_size.x() as f32 / w, // stretch to full width/height
        yfact: window_size.y() as f32 / h,
        xmin: bbox.0 as f32,
        ymax: bbox.3 as f32,
        canvas: &mut canvas,
        path: Path2D::new(),
    };
    let mut freader =
        FeatureReader::select_bbox(&mut file, &header, bbox.0, bbox.1, bbox.2, bbox.3)?;
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
