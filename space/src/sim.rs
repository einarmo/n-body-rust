use std::fmt::Display;

use cgmath::{InnerSpace, Point3, Vector3, Zero};
use rayon::{ThreadPool, ThreadPoolBuilder};

// Average distance between earth and the sun, in meters
pub const AU: f64 = 1.495e11;
// Mass of earth, in kilograms
pub const M0: f64 = 5.972e24;

pub const G_ABS: f64 = 6.674e-11;
// Adjusted gravitational constant in earth masses and AU
pub const G: f64 = G_ABS * M0 / (AU * AU * AU);
// Seconds per computation (really!)
pub const DELTA: f64 = 10.0;
// Padding between all objects to avoid division by zero, 10 meters.
// pub const COLLISION_EPSILON: f64 = (10.0 / AU) * (10.0 / AU);
pub const COLLISION_EPSILON: f64 = 0.0;

pub const _TEST: f64 = G * 333000.0;
pub const _SPEED: f64 = 29.8e3 / AU;
pub const _REF: f64 = 6.674e-11 * M0 * 333000.0 / (AU * AU);
pub const _REF2: f64 = _REF / AU;

#[derive(Debug)]
pub struct ObjectInfo {
    pub pos: Point3<f64>,
    pub vel: Vector3<f64>,
    pub mass: f64,
}

impl ObjectInfo {
    pub fn get_acc_towards(&self, other: &ObjectInfo, out: &mut Vector3<f64>) {
        let rel = other.pos - self.pos;
        *out += rel * other.mass * G / (rel.magnitude2() * rel.magnitude() + COLLISION_EPSILON);
    }
}

pub struct ObjectBuffer {
    pub objects: Vec<ObjectInfo>,
    out_buffer: Vec<Vector3<f64>>,
    n_threads: usize,
    pool: ThreadPool,
}

const MAX_THREADS: usize = 4;
const OBJECTS_PER_THREAD: usize = 10;

pub fn compute_target_threads(n_objects: usize) -> usize {
    assert!(n_objects > 0);
    (((n_objects as f32) / (OBJECTS_PER_THREAD as f32)).ceil() as usize).min(MAX_THREADS)
}

fn iter_chunk(objects: &[ObjectInfo], out_buffer: &mut [Vector3<f64>], start: usize) {
    let range = start..(start + out_buffer.len());
    debug_assert!(range.end <= objects.len());
    for (idx, i) in range.enumerate() {
        let obj = &objects[i];
        let out = &mut out_buffer[idx];
        for (other_idx, other) in objects.iter().enumerate() {
            if other_idx == i {
                continue;
            }
            obj.get_acc_towards(other, out);
        }
    }
}

impl ObjectBuffer {
    pub fn new(objects: Vec<ObjectInfo>) -> Self {
        let len = objects.len();
        let out_buffer = vec![Vector3::<f64>::zero(); len];
        let n_threads = compute_target_threads(objects.len());

        Self {
            objects,
            out_buffer,
            n_threads,
            pool: ThreadPoolBuilder::new()
                .num_threads(n_threads)
                .build()
                .unwrap(),
        }
    }

    pub fn exec_iter(&mut self) {
        let mut per_thread = self.objects.len() / self.n_threads;
        if self.objects.len() % self.n_threads > 0 {
            per_thread += 1;
        }
        self.pool
            .install(|| exec_iter_rec(&self.objects, &mut self.out_buffer, per_thread, 0));
        for (obj, acc) in self.objects.iter_mut().zip(self.out_buffer.iter_mut()) {
            obj.vel += *acc * DELTA;
            obj.pos += obj.vel * DELTA;
            // println!("{:?}: {}", obj.pos, acc.magnitude());
            acc.x = 0.0;
            acc.y = 0.0;
            acc.z = 0.0;
        }
    }
}

fn exec_iter_rec(
    objects: &[ObjectInfo],
    out_buffer: &mut [Vector3<f64>],
    per_thread: usize,
    idx: usize,
) {
    let next = (idx + 1) * per_thread;
    if next >= objects.len() {
        iter_chunk(objects, out_buffer, idx * per_thread);
    } else {
        let (slice, next) = out_buffer.split_at_mut(next);
        rayon::join(
            || {
                iter_chunk(objects, slice, idx * per_thread);
            },
            || {
                exec_iter_rec(objects, next, per_thread, idx + 1);
            },
        );
    }
}

const SEC_PER_HOUR: f64 = 60.0 * 60.0;
const SEC_PER_DAY: f64 = SEC_PER_HOUR * 24.0;
const SEC_PER_YEAR: f64 = 365.25 * SEC_PER_DAY;

pub struct ElapsedTime {
    pub years: u64,
    pub days: u64,
    pub hours: u64,
    pub minutes: u64,
    pub seconds: f64,
    pub ticks: u64,
}

impl Display for ElapsedTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}Y {}D {}:{}:{} ({} ticks)",
            self.years, self.days, self.hours, self.minutes, self.seconds, self.ticks
        )
    }
}

pub fn compute_elapsed_time(ticks: u64) -> ElapsedTime {
    let mut time_s = (ticks as f64) * DELTA;

    let years = (time_s / SEC_PER_YEAR).floor();
    time_s -= years * SEC_PER_YEAR;
    let days = (time_s / SEC_PER_DAY).floor();
    time_s -= days * SEC_PER_DAY;
    let hours = (time_s / SEC_PER_HOUR).floor();
    time_s -= hours * SEC_PER_HOUR;
    let minutes = (time_s / 60.0).floor();
    let seconds = time_s - minutes * 60.0;

    ElapsedTime {
        years: years as u64,
        days: days as u64,
        hours: hours as u64,
        minutes: minutes as u64,
        seconds,
        ticks,
    }
}
