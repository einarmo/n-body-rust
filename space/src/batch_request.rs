use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use std::sync::Mutex;

use crate::objects::Objects;
use crate::sim::{DELTA, ObjectBuffer};

/// Primitive for communicating between simulation and graphics.
pub struct BatchRequest {
    sample: Mutex<Vec<[f32; 3]>>,
    should_sample: AtomicBool,
    simulation_tick: AtomicU64,
    delta: AtomicU64,
}

impl BatchRequest {
    pub fn new(n_objects: usize) -> Self {
        Self {
            sample: Mutex::new(vec![[0.0, 0.0, 0.0]; n_objects]),
            should_sample: AtomicBool::new(true),
            simulation_tick: AtomicU64::new(0),
            delta: AtomicU64::new(DELTA.to_bits()),
        }
    }

    pub fn delta(&self) -> f64 {
        f64::from_bits(self.delta.load(Ordering::Relaxed))
    }

    pub fn set_delta(&self, rate: f64) {
        self.delta.store(rate.to_bits(), Ordering::Relaxed);
    }

    /// Return whether we are ready to a accept a new simulation batch.
    pub fn should_store(&self) -> bool {
        self.should_sample
            .compare_exchange_weak(true, false, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    }

    /// Store a sample of each simulated object, as well as the current tick.
    pub fn store(&self, sim: &ObjectBuffer, tick: u64) {
        self.simulation_tick.store(tick, Ordering::Relaxed);
        let mut data = self.sample.lock().unwrap();
        for (buff, obj) in data.iter_mut().zip(sim.objects.iter()) {
            buff[0] = obj.pos.x as f32;
            buff[1] = obj.pos.y as f32;
            buff[2] = obj.pos.z as f32;
        }
    }

    /// Retrieve a sample, and request a new one from the simulation.
    pub fn sample(&self, objects: &mut Objects) {
        let data = self.sample.lock().unwrap();
        objects.push_items(&data);
        self.should_sample.store(true, Ordering::Relaxed);
    }

    pub fn current_ticks(&self) -> u64 {
        self.simulation_tick.load(Ordering::Relaxed)
    }
}
