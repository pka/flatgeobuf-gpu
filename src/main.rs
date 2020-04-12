use flatgeobuf::*;
use geozero_api::GeomProcessor;
use pathfinder_canvas::{Canvas, CanvasFontContext, CanvasRenderingContext2D, Path2D};
use pathfinder_color::{rgbu, ColorF};
use pathfinder_content::fill::FillRule;
use pathfinder_geometry::vector::{vec2f, vec2i, Vector2F, Vector2I};
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
use std::time::Instant;

mod ui;

const DEFAULT_WINDOW_WIDTH: i32 = 1067;
const DEFAULT_WINDOW_HEIGHT: i32 = 800;

fn main() -> std::result::Result<(), std::io::Error> {
    // Set up SDL2.
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();

    // Make sure we have at least a GL 3.0 context. Pathfinder requires this.
    let gl_attributes = video.gl_attr();
    gl_attributes.set_context_profile(GLProfile::Core);
    gl_attributes.set_context_version(3, 3);

    // Open a window.
    let window_size = vec2i(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT);
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
    let renderer = Renderer::new(
        GLDevice::new(GLVersion::GL3, 0),
        &EmbeddedResourceLoader::new(),
        DestFramebuffer::full_window(window_size),
        RendererOptions {
            background_color: Some(ColorF::white()),
        },
    );

    let mut fgb_renderer = FgbRenderer::new(renderer, window_size, vec2f(8.53, 47.37));
    fgb_renderer.render()?;
    window.gl_swap_window();

    // Enter main render loop.
    let mut event_pump = sdl_context.event_pump().unwrap();
    loop {
        let ev = event_pump.wait_event();
        match fgb_renderer.handle_event(ev) {
            AppEvent::Idle => {}
            AppEvent::Redraw => {
                fgb_renderer.render()?;
                window.gl_swap_window();
            }
            AppEvent::Quit => return Ok(()),
        }
    }
}

struct FgbRenderer {
    renderer: Renderer<GLDevice>,
    scene: SceneProxy,
    window_size: Vector2I,
    center: Vector2F,
    /// Size of center pixel in map coordinates
    pixel_size: Vector2F,
}

enum AppEvent {
    Idle,
    Redraw,
    Quit,
}

struct PathDrawer<'a> {
    xmin: f32,
    ymax: f32,
    pixel_size: Vector2F,
    canvas: &'a mut CanvasRenderingContext2D,
    path: Path2D,
}

impl<'a> GeomProcessor for PathDrawer<'a> {
    fn pointxy(&mut self, x: f64, y: f64, idx: usize) {
        // x,y are in degrees, y must be inverted
        let x = (x as f32 - self.xmin) / self.pixel_size.x();
        let y = (self.ymax - y as f32) / self.pixel_size.y();
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

impl FgbRenderer {
    fn new(renderer: Renderer<GLDevice>, window_size: Vector2I, center: Vector2F) -> FgbRenderer {
        let pixel_size = vec2f(0.00003, 0.00003); // TODO: calculate from scale and center
        FgbRenderer {
            renderer,
            scene: SceneProxy::new(RayonExecutor),
            window_size,
            center,
            pixel_size,
        }
    }

    fn render(&mut self) -> std::result::Result<(), std::io::Error> {
        let font_context = CanvasFontContext::from_system_source();
        let stats_ui_presenter = ui::StatsUIPresenter::new(
            &self.renderer.device,
            &EmbeddedResourceLoader::new(),
            self.window_size,
        );

        let mut canvas = Canvas::new(self.window_size.to_f32()).get_context_2d(font_context);

        canvas.set_line_width(1.0);
        canvas.set_fill_style(rgbu(132, 132, 132));

        let mut stats = ui::Stats::default();
        let start = Instant::now();
        let mut file = BufReader::new(File::open(
            "/home/pi/code/gis/flatgeobuf/test/data/osm/osm-buildings-ch.fgb",
            // "/home/pi/code/gis/flatgeobuf/test/data/countries.fgb",
        )?);
        let hreader = HeaderReader::read(&mut file)?;
        let header = hreader.header();

        let wsize = vec2f(self.window_size.x() as f32, self.window_size.y() as f32);
        // let bbox = (-180.0, -90.0, 180.0, 90.0);
        let bbox = (
            self.center.x() - wsize.x() / 2.0 * self.pixel_size.x(),
            self.center.y() - wsize.y() / 2.0 * self.pixel_size.y(),
            self.center.x() + wsize.x() / 2.0 * self.pixel_size.x(),
            self.center.y() + wsize.y() / 2.0 * self.pixel_size.y(),
        );

        let mut drawer = PathDrawer {
            xmin: bbox.0,
            ymax: bbox.3,
            pixel_size: self.pixel_size,
            canvas: &mut canvas,
            path: Path2D::new(),
        };
        let mut freader = FeatureReader::select_bbox(
            &mut file,
            &header,
            bbox.0 as f64,
            bbox.1 as f64,
            bbox.2 as f64,
            bbox.3 as f64,
        )?;
        stats.fbg_index_read_time = start.elapsed();
        stats.feature_count = freader.filter_count().unwrap();
        let start = Instant::now();
        while let Ok(feature) = freader.next(&mut file) {
            let geometry = feature.geometry().unwrap();
            geometry.process(&mut drawer, header.geometry_type());
        }
        stats.fbg_data_read_time = start.elapsed();

        // Render the canvas to screen.
        let start = Instant::now();
        self.scene.replace_scene(canvas.into_canvas().into_scene());
        self.scene
            .build_and_render(&mut self.renderer, BuildOptions::default());
        stats.render_time = start.elapsed();

        stats_ui_presenter.draw_stats_window(&self.renderer.device, &stats);

        Ok(())
    }

    fn handle_event(&mut self, event: Event) -> AppEvent {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => {
                return AppEvent::Quit;
            }
            Event::MouseMotion {
                xrel,
                yrel,
                mousestate,
                ..
            } => {
                if mousestate.left() {
                    self.move_center(xrel, yrel);
                }
            }
            Event::MouseButtonUp { x: _, y: _, .. } => {
                return AppEvent::Redraw;
            }
            _ => {}
        }
        AppEvent::Idle
    }

    fn move_center(&mut self, xrel: i32, yrel: i32) {
        self.center += vec2f(
            xrel as f32 * -self.pixel_size.x(),
            yrel as f32 * self.pixel_size.y(),
        );
    }
}
