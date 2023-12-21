use cgmath::{InnerSpace, Point3, Vector3, Zero};
use rayon::{ThreadPool, ThreadPoolBuilder};

// Average distance between earth and the sun, in meters
pub const AU: f64 = 1.495e11;
// Mass of earth, in kilograms
pub const M0: f64 = 5.972e24;
// Adjusted gravitational constant in earth masses and AU
pub const G: f64 = 6.674e-11 * M0 / (AU * AU * AU);
// Seconds per computation (really!)
pub const DELTA: f64 = 10.0;
// Padding between all objects to avoid division by zero, 10 meters.
pub const COLLISION_EPSILON: f64 = (10.0 / AU) * (10.0 / AU);

pub const _TEST: f64 = G * 333000.0;
pub const _SPEED: f64 = 29.8e3 / AU;
pub const _REF: f64 = 6.674e-11 * M0 * 333000.0 / (AU * AU);
pub const _REF2: f64 = _REF / AU;

pub struct ObjectInfo {
    pub pos: Point3<f64>,
    pub vel: Vector3<f64>,
    pub mass: f64,
}

impl ObjectInfo {
    pub fn get_acc_towards(&self, other: &ObjectInfo, out: &mut Vector3<f64>) {
        let rel = other.pos - self.pos;
        *out += rel * other.mass * G / (rel.magnitude() + COLLISION_EPSILON);
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
    assert!(range.end <= objects.len());
    let mut idx = 0;
    for i in range {
        let obj = &objects[i];
        let out = &mut out_buffer[idx];
        idx += 1;
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
