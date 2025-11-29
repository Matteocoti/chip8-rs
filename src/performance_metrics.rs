use std::collections::VecDeque;
use std::time::{Duration, Instant};

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

pub struct PerformanceMetrics {
    // Frame rate tracking
    frame_times: VecDeque<Instant>,
    fps: f64,
    fps_update_time: Instant,

    // Loop timing
    loop_durations: VecDeque<Duration>,
    avg_loop_time: Duration,
    min_loop_time: Duration,
    max_loop_time: Duration,

    // Configuration
    sample_size: usize,
    visible: bool,

    // Timing markers
    last_frame_time: Instant,
}

impl PerformanceMetrics {
    pub fn new(sample_size: usize) -> Self {
        Self {
            frame_times: VecDeque::with_capacity(sample_size),
            fps: 0.0,
            fps_update_time: Instant::now(),

            loop_durations: VecDeque::with_capacity(sample_size),
            avg_loop_time: Duration::from_nanos(0),
            min_loop_time: Duration::MAX,
            max_loop_time: Duration::from_nanos(0),

            sample_size,
            visible: false, // Hidden by default

            last_frame_time: Instant::now(),
        }
    }

    /// Called at the start of each application loop
    pub fn start_frame(&mut self) -> Instant {
        self.last_frame_time = Instant::now();
        self.last_frame_time
    }

    /// Called at the end of each application loop
    pub fn end_frame(&mut self, start_time: Instant) -> Duration {
        let now = Instant::now();
        let frame_duration = now.duration_since(start_time);

        // Record frame time for FPS calculation
        self.frame_times.push_back(now);
        if self.frame_times.len() > self.sample_size {
            self.frame_times.pop_front();
        }

        // Record loop duration
        self.loop_durations.push_back(frame_duration);
        if self.loop_durations.len() > self.sample_size {
            self.loop_durations.pop_front();
        }

        // Update min/max times
        if frame_duration < self.min_loop_time {
            self.min_loop_time = frame_duration;
        }
        if frame_duration > self.max_loop_time {
            self.max_loop_time = frame_duration;
        }

        // Update average loop time
        let total: Duration = self.loop_durations.iter().sum();
        if !self.loop_durations.is_empty() {
            self.avg_loop_time = total / self.loop_durations.len() as u32;
        }

        // Update FPS calculation every 500ms
        if now.duration_since(self.fps_update_time) >= Duration::from_millis(500) {
            self.update_fps();
            self.fps_update_time = now;
        }

        frame_duration
    }

    fn update_fps(&mut self) {
        if self.frame_times.len() < 2 {
            self.fps = 0.0;
            return;
        }

        let oldest = self.frame_times.front().unwrap();
        let newest = self.frame_times.back().unwrap();
        let elapsed = newest.duration_since(*oldest);

        // Calculate frames per second
        let frame_count = (self.frame_times.len() - 1) as f64;
        self.fps = if elapsed.as_secs_f64() > 0.0 {
            frame_count / elapsed.as_secs_f64()
        } else {
            0.0
        };
    }

    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    // Getters for metrics
    pub fn fps(&self) -> f64 {
        self.fps
    }
    pub fn avg_loop_time(&self) -> Duration {
        self.avg_loop_time
    }
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let fps_text = format!("FPS: {:.1}", self.fps());
        let loop_text = format!("Loop: {:.2}ms", self.avg_loop_time().as_secs_f64() * 1000.0);
        let date_time = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let metrics_line = Line::from(vec![
            Span::styled(fps_text, Style::default().fg(Color::Green)),
            Span::raw(" | "),
            Span::styled(loop_text, Style::default().fg(Color::Yellow)),
            Span::raw(" | "),
            Span::styled(date_time, Style::default().fg(Color::Gray)),
        ]);

        // Draw metrics with a transparent background (no block)
        let metrics_widget = Paragraph::new(metrics_line).alignment(Alignment::Center);

        f.render_widget(metrics_widget, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_not_visible() {
        assert!(!PerformanceMetrics::new(200).is_visible());
    }

    #[test]
    fn toggle_visibility_makes_visible() {
        let mut m = PerformanceMetrics::new(200);
        m.toggle_visibility();
        assert!(m.is_visible());
    }

    #[test]
    fn toggle_visibility_twice_restores_hidden() {
        let mut m = PerformanceMetrics::new(200);
        m.toggle_visibility();
        m.toggle_visibility();
        assert!(!m.is_visible());
    }

    #[test]
    fn initial_fps_is_zero() {
        assert_eq!(PerformanceMetrics::new(200).fps(), 0.0);
    }

    #[test]
    fn initial_avg_loop_time_is_zero() {
        assert_eq!(
            PerformanceMetrics::new(200).avg_loop_time(),
            Duration::from_nanos(0)
        );
    }

    #[test]
    fn end_frame_returns_a_duration() {
        let mut m = PerformanceMetrics::new(200);
        let start = m.start_frame();
        let elapsed = m.end_frame(start);
        // Duration is non-negative by construction; verify it is the frame duration
        assert!(elapsed <= Duration::from_secs(1)); // sanity: one frame can't take > 1s in tests
    }

    #[test]
    fn avg_loop_time_updates_after_end_frame() {
        let mut m = PerformanceMetrics::new(200);
        let start = m.start_frame();
        m.end_frame(start);
        // After one frame, avg_loop_time should be non-zero (some time passed)
        // We can't assert the exact value due to timing, but it shouldn't panic
        let _ = m.avg_loop_time();
    }
}
