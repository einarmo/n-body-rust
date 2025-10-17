use std::fmt::Display;

use cgmath::{InnerSpace, Point3, Vector3};
use rayon::ThreadPool;

use crate::constants::{COLLISION_EPSILON, G};

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
        other_pos: &Point3<f64>,
        other_mass: f64,
        out: &mut Vector3<f64>,
    ) {
        let rel = *other_pos - self.pos;
        *out += rel * other_mass * G / (rel.magnitude2() * rel.magnitude() + COLLISION_EPSILON);
    }
}

pub struct ObjectBuffer {
    pub objects: Vec<ObjectInfo>,
    out_buffer: Vec<Vector3<f64>>,
    per_thread: usize,
    pool: ThreadPool,
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
