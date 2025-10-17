use cgmath::{Vector3, Zero};
use rayon::ThreadPoolBuilder;

use crate::{
    constants::{MAX_THREADS, OBJECTS_PER_THREAD, USE_BARNES_HUT},
    sim::{ObjectBuffer, ObjectInfo, barnes_hut},
};

pub fn compute_target_threads(n_objects: usize) -> usize {
    assert!(n_objects > 0);
    n_objects.div_ceil(OBJECTS_PER_THREAD).min(MAX_THREADS)
}

impl ObjectBuffer {
    pub fn new(objects: Vec<ObjectInfo>) -> Self {
        let len = objects.len();
        let out_buffer = vec![Vector3::<f64>::zero(); len];
        let n_threads = compute_target_threads(objects.len());

        Self {
            per_thread: objects.len().div_ceil(n_threads),
            objects,
            out_buffer,
            pool: ThreadPoolBuilder::new()
                .num_threads(n_threads)
                .build()
                .unwrap(),
        }
    }

    pub fn exec_iter(&mut self, delta: f64) {
        // Number of objects per thread is equal to ceil[num_objects / num_threads]
        if USE_BARNES_HUT {
            barnes_hut::iter(&mut self.objects, &mut self.out_buffer, 1.0);
            self.pool.install(|| {
                add_acc_rec(
                    &mut self.objects,
                    &mut self.out_buffer,
                    self.per_thread,
                    0,
                    delta,
                )
            });
        } else {
            self.pool
                .install(|| exec_iter_rec(&self.objects, &mut self.out_buffer, self.per_thread, 0));
            self.pool.install(|| {
                add_acc_rec(
                    &mut self.objects,
                    &mut self.out_buffer,
                    self.per_thread,
                    0,
                    delta,
                )
            });
        }
    }
}

fn add_acc(objects: &mut [ObjectInfo], acc: &mut [Vector3<f64>], delta: f64) {
    for (obj, acc) in objects.iter_mut().zip(acc.iter_mut()) {
        // Integrate the acceleration by multiplying it with the time step
        // and add it to the velocity
        obj.vel += *acc * delta;
        // Integrate the velocity by multiplying it with the time step
        // and add it to the position
        obj.pos += obj.vel * delta;
        // We keep the acceleration object for the next iteration, but we need to reset it.
        acc.x = 0.0;
        acc.y = 0.0;
        acc.z = 0.0;
    }
}

fn add_acc_rec(
    objects: &mut [ObjectInfo],
    acc: &mut [Vector3<f64>],
    per_thread: usize,
    idx: usize,
    delta: f64,
) {
    if per_thread >= objects.len() {
        add_acc(objects, acc, delta);
    } else {
        let (obj_slice, next_obj) = objects.split_at_mut(per_thread);
        let (acc_slice, next_acc) = acc.split_at_mut(per_thread);
        rayon::join(
            || {
                add_acc_rec(obj_slice, acc_slice, per_thread, idx, delta);
            },
            || {
                add_acc_rec(next_obj, next_acc, per_thread, idx + 1, delta);
            },
        );
    }
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

fn exec_iter_rec(
    objects: &[ObjectInfo],
    out_buffer: &mut [Vector3<f64>],
    per_thread: usize,
    idx: usize,
) {
    if per_thread >= out_buffer.len() {
        iter_chunk(objects, out_buffer, idx * per_thread);
    } else {
        let (slice, next) = out_buffer.split_at_mut(per_thread);
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
