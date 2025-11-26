use std::fmt::Display;

use cgmath::{InnerSpace, Point3, Vector3, Zero};
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::{
    constants::{COLLISION_EPSILON, G, MAX_THREADS, OBJECTS_PER_THREAD},
    sim::direct::par_add_rec,
};

pub mod barnes_hut;
mod direct;

#[derive(Debug, Clone)]
pub struct ObjectInfo {
    pub pos: Point3<f64>,
    pub vel: Vector3<f64>,
    pub mass: f64,
}

impl ObjectInfo {
    #[inline]
    pub fn get_acc_towards(&self, other: &ObjectInfo, out: &mut Vector3<f64>) {
        let rel = other.pos - self.pos;
        *out += rel * other.mass * G / (rel.magnitude2() * rel.magnitude() + COLLISION_EPSILON);
    }

    #[inline]
    pub fn get_acc_towards_raw(
        &self,
        other_mass: f64,
        rel: Vector3<f64>,
        mag_sq: f64,
        out: &mut Vector3<f64>,
    ) {
        *out += rel * other_mass * G / (mag_sq * mag_sq.sqrt() + COLLISION_EPSILON);
    }
}

fn compute_target_threads(n_objects: usize) -> usize {
    assert!(n_objects > 0);
    n_objects.div_ceil(OBJECTS_PER_THREAD).min(MAX_THREADS)
}

impl<R: SimulationImpl + Send> ObjectBuffer<R> {
    pub fn new(objects: Vec<ObjectInfo>, simulation: R) -> Self {
        let len = objects.len();
        let out_buffer = vec![Vector3::<f64>::zero(); len];
        let n_threads = compute_target_threads(objects.len());

        Self {
            objects,
            out_buffer,
            pool: ThreadPoolBuilder::new()
                .num_threads(n_threads)
                .build()
                .unwrap(),
            simulation,
        }
    }

    pub fn exec_iter(&mut self, delta: f64) {
        // Number of objects per thread is equal to ceil[num_objects / num_threads]
        self.pool.install(|| {
            self.simulation
                .iter(&mut self.objects, &mut self.out_buffer);
            par_add_rec(&mut self.objects, &mut self.out_buffer, delta);
        });
    }
}

pub trait SimulationImpl {
    fn iter(&mut self, objects: &mut [ObjectInfo], out_buffer: &mut [Vector3<f64>]);

    fn iter_single_threaded(&mut self, objects: &mut [ObjectInfo], out_buffer: &mut [Vector3<f64>]);
}

pub struct BarnesHutSim {
    pub theta: f64,
    pub tree: barnes_hut::FmmTree,
}

impl BarnesHutSim {
    pub fn new(theta: f64) -> Self {
        Self {
            theta,
            tree: barnes_hut::FmmTree::new(),
        }
    }
}

impl SimulationImpl for BarnesHutSim {
    fn iter(&mut self, objects: &mut [ObjectInfo], out_buffer: &mut [Vector3<f64>]) {
        barnes_hut::iter(objects, out_buffer, &mut self.tree, self.theta);
    }

    fn iter_single_threaded(
        &mut self,
        objects: &mut [ObjectInfo],
        out_buffer: &mut [Vector3<f64>],
    ) {
        barnes_hut::iter_single_threaded(objects, out_buffer, &mut self.tree, self.theta);
    }
}

pub struct BruteForceSim;

impl SimulationImpl for BruteForceSim {
    fn iter(&mut self, objects: &mut [ObjectInfo], out_buffer: &mut [Vector3<f64>]) {
        direct::iter(objects, out_buffer);
    }

    fn iter_single_threaded(
        &mut self,
        objects: &mut [ObjectInfo],
        out_buffer: &mut [Vector3<f64>],
    ) {
        direct::iter_single_threaded(objects, out_buffer);
    }
}

pub struct ObjectBuffer<R> {
    pub objects: Vec<ObjectInfo>,
    out_buffer: Vec<Vector3<f64>>,
    pool: ThreadPool,
    simulation: R,
}

const SEC_PER_HOUR: f64 = 60.0 * 60.0;
const SEC_PER_DAY: f64 = SEC_PER_HOUR * 24.0;
const SEC_PER_YEAR: f64 = 365.25 * SEC_PER_DAY;

#[derive(Default)]
pub struct ElapsedTime {
    pub years: u64,
    pub days: u64,
    pub hours: u64,
    pub minutes: u64,
    pub seconds: f64,
    pub ticks: f64,
}

impl Display for ElapsedTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}Y {}D {:0>2}:{:0>2}:{:0>2} ({} ticks)",
            self.years, self.days, self.hours, self.minutes, self.seconds, self.ticks
        )
    }
}

pub fn compute_elapsed_time(ticks: f64, delta: f64) -> ElapsedTime {
    let mut time_s = ticks * delta;

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
