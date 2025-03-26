use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use std::sync::Mutex;

use crate::objects::Objects;
use crate::sim::ObjectBuffer;

pub struct BatchRequest {
    sample: Mutex<Vec<[f32; 3]>>,
    should_sample: AtomicBool,
    simulation_tick: AtomicU64,
}

impl BatchRequest {
    pub fn new(n_objects: usize) -> Self {
        Self {
            sample: Mutex::new(vec![[0.0, 0.0, 0.0]; n_objects]),
            should_sample: AtomicBool::new(true),
            simulation_tick: AtomicU64::new(0),
        }
    }

    pub fn should_store(&self) -> bool {
        self.should_sample
            .compare_exchange_weak(true, false, Ordering::Acquire, Ordering::SeqCst)
            .is_ok()
    }

    pub fn store(&self, sim: &ObjectBuffer, tick: u64) {
        self.simulation_tick.store(tick, Ordering::Relaxed);
        let mut data = self.sample.lock().unwrap();
        for (buff, obj) in data.iter_mut().zip(sim.objects.iter()) {
            buff[0] = obj.pos.x as f32;
            buff[1] = obj.pos.y as f32;
            buff[2] = obj.pos.z as f32;
        }
    }

    pub fn sample(&self, objects: &mut Objects) {
        let data = self.sample.lock().unwrap();
        objects.push_items(&data);
        self.should_sample.store(true, Ordering::SeqCst);
    }

    pub fn current_ticks(&self) -> u64 {
        self.simulation_tick.load(Ordering::Relaxed)
    }
}
