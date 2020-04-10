// Based on pathfinder/renderer/src/gpu/debug.rs

use pathfinder_geometry::rect::RectI;
use pathfinder_geometry::vector::{vec2i, Vector2I};
use pathfinder_gpu::Device;
use pathfinder_resources::ResourceLoader;
use pathfinder_ui::{UIPresenter, FONT_ASCENT, LINE_HEIGHT, PADDING, WINDOW_COLOR};
use std::time::Duration;

const STATS_WINDOW_WIDTH: i32 = 330;
const STATS_WINDOW_HEIGHT: i32 = LINE_HEIGHT * 4 + PADDING + 2;

#[derive(Clone, Default)]
pub struct Stats {
    pub feature_count: usize,
    pub fbg_index_read_time: Duration,
    pub fbg_data_read_time: Duration,
    pub render_time: Duration,
}

pub struct StatsUIPresenter<D>
where
    D: Device,
{
    pub ui_presenter: UIPresenter<D>,
}

impl<D> StatsUIPresenter<D>
where
    D: Device,
{
    pub fn new(
        device: &D,
        resources: &dyn ResourceLoader,
        framebuffer_size: Vector2I,
    ) -> StatsUIPresenter<D> {
        let ui_presenter = UIPresenter::new(device, resources, framebuffer_size);
        StatsUIPresenter { ui_presenter }
    }

    pub fn draw_stats_window(&self, device: &D, stats: &Stats) {
        let framebuffer_size = self.ui_presenter.framebuffer_size();
        let bottom = framebuffer_size.y() - PADDING;
        let window_rect = RectI::new(
            vec2i(
                framebuffer_size.x() - PADDING - STATS_WINDOW_WIDTH,
                bottom - PADDING - STATS_WINDOW_HEIGHT,
            ),
            vec2i(STATS_WINDOW_WIDTH, STATS_WINDOW_HEIGHT),
        );

        self.ui_presenter
            .draw_solid_rounded_rect(device, window_rect, WINDOW_COLOR);

        let origin = window_rect.origin() + vec2i(PADDING, PADDING + FONT_ASCENT);
        self.ui_presenter.draw_text(
            device,
            &format!("Features: {}", stats.feature_count),
            origin,
            false,
        );
        self.ui_presenter.draw_text(
            device,
            &format!(
                "Index read: {:.1} ms",
                duration_to_ms(stats.fbg_index_read_time)
            ),
            origin + vec2i(0, LINE_HEIGHT * 1),
            false,
        );
        self.ui_presenter.draw_text(
            device,
            &format!(
                "Data read: {:.1} ms",
                duration_to_ms(stats.fbg_data_read_time)
            ),
            origin + vec2i(0, LINE_HEIGHT * 2),
            false,
        );
        self.ui_presenter.draw_text(
            device,
            &format!("Rendering: {:.1} ms", duration_to_ms(stats.render_time)),
            origin + vec2i(0, LINE_HEIGHT * 3),
            false,
        );
    }
}

fn duration_to_ms(time: Duration) -> f64 {
    time.as_secs() as f64 * 1000.0 + time.subsec_nanos() as f64 / 1000000.0
}
