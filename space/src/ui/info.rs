use std::time::Instant;

use eframe::egui;

use crate::{
    camera::Camera,
    objects::Objects,
    sim::{ElapsedTime, compute_elapsed_time},
};

pub struct InfoPanel {
    pub last_tick: u64,
    pub last_update: Instant,
    pub tick_rates: [f64; 30],
    pub tick_rate_index: usize,

    pub last_time: ElapsedTime,
    pub last_time_per_second: ElapsedTime,
}

impl InfoPanel {
    pub fn new() -> Self {
        Self {
            last_tick: 0,
            last_update: Instant::now(),
            tick_rates: [0.0; 30],
            tick_rate_index: 0,

            last_time: ElapsedTime::default(),
            last_time_per_second: ElapsedTime::default(),
        }
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        objects: &Objects,
        tick: u64,
        camera: &Camera,
        ui_tick: u32,
        delta: f64,
    ) {
        let upd_time = Instant::now();
        let elapsed = upd_time.duration_since(self.last_update);
        let ticks_elapsed = tick - self.last_tick;

        self.tick_rates[self.tick_rate_index] = (ticks_elapsed as f64) / elapsed.as_secs_f64();
        self.tick_rate_index = (self.tick_rate_index + 1) % self.tick_rates.len();

        self.last_tick = tick;
        self.last_update = upd_time;

        let avg_tick_rate = self.tick_rates.iter().sum::<f64>() / self.tick_rates.len() as f64;

        ui.vertical(|ui| {
            if ui_tick % 10 == 0 {
                self.last_time = compute_elapsed_time(tick as f64);
                self.last_time_per_second = compute_elapsed_time(avg_tick_rate);
            }
            ui.label(format!("Current time: {}", self.last_time));
            ui.label(format!(
                "Simulated time per second: {}",
                self.last_time_per_second
            ));
            ui.label(format!(
                "Current time per tick: {}",
                compute_elapsed_time(delta)
            ));

            if let Some(focus) = camera.focus()
                && let Some(desc) = objects.objects().get(focus as usize)
            {
                ui.label(format!("Focused object: {}", desc.name));
            }
        });
    }
}
