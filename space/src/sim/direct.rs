use cgmath::Vector3;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};

use crate::sim::ObjectInfo;

pub fn par_add_rec(objects: &mut [ObjectInfo], acc: &mut [Vector3<f64>], delta: f64) {
    objects
        .par_iter_mut()
        .zip(acc.par_iter_mut())
        .for_each(|(obj, acc)| {
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
        });
}

pub fn iter(objects: &mut [ObjectInfo], out_buffer: &mut [Vector3<f64>]) {
    objects
        .par_iter()
        .zip(out_buffer.par_iter_mut())
        .enumerate()
        .for_each(|(i, (obj, out))| {
            for (other_idx, other) in objects.iter().enumerate() {
                if other_idx == i {
                    continue;
                }
                obj.get_acc_towards(other, out);
            }
        });
}
