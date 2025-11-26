mod perf;

use cgmath::{Point3, Vector3};
use criterion::{Criterion, criterion_group, criterion_main};
use space::{ObjectInfo, SimulationImpl, constants::AU};

fn gen_random(count: usize) -> (Vec<ObjectInfo>, Vec<Vector3<f64>>) {
    let mut objs = Vec::new();
    for _ in 0..count {
        objs.push(ObjectInfo {
            pos: Point3::new(
                rand::random_range(-1e1..1e1),
                rand::random_range(-1e1..1e1),
                rand::random_range(-1e1..1e1),
            ),
            vel: Vector3::new(
                rand::random_range(-1e3..1e3) / AU,
                rand::random_range(-1e3..1e3) / AU,
                rand::random_range(-1e3..1e3) / AU,
            ),
            mass: rand::random_range(1000.0..1000000.0),
        });
    }

    let out_buffer = vec![Vector3::<f64>::new(0.0, 0.0, 0.0); objs.len()];

    (objs, out_buffer)
}

fn bench_barnes_hut_random(c: &mut Criterion) {
    let (mut objs, mut out_buffer) = gen_random(1000);

    c.bench_function("barnes_hut_random_1k", |b| {
        b.iter(|| {
            let mut sim = space::BarnesHutSim::new(0.5);
            sim.iter_single_threaded(&mut objs, &mut out_buffer);
        })
    });
}

#[allow(unused)]
fn bench_barnes_hut_random_par(c: &mut Criterion) {
    // This bench is rather unstable.
    let (mut objs, mut out_buffer) = gen_random(1000);

    c.bench_function("barnes_hut_random_1k_par", |b| {
        b.iter(|| {
            let mut sim = space::BarnesHutSim::new(0.5);
            sim.iter(&mut objs, &mut out_buffer);
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(perf::FlamegraphProfiler::new(100));
    targets = bench_barnes_hut_random/* bench_barnes_hut_random_par */
}
criterion_main!(benches);
