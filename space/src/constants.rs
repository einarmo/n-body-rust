// PHYSICAL
/// Average distance between earth and the sun, in meters
pub const AU: f64 = 1.495e11;
/// Mass of earth, in kilograms
pub const M0: f64 = 5.972e24;
/// SI gravitational constant, in m^3 kg^-1 s^-2
pub const G_ABS: f64 = 6.674e-11;
/// Adjusted gravitational constant in earth masses and AU
pub const G: f64 = G_ABS * M0 / (AU * AU * AU);
/// Seconds per computation (really!). Legacy only.
pub const DELTA: f64 = 10.0;
/// Padding between all objects to avoid division by zero, 10 meters.
pub const COLLISION_EPSILON: f64 = 1e-15;

// SIMULATION
/// Hard cap on number of threads to use.
pub const MAX_THREADS: usize = 20;
/// Minimum number of objects per thread.
pub const OBJECTS_PER_THREAD: usize = 2000;
/// Interval in ticks
pub const CHECK_INTERVAL: u64 = 1;
/// 30 seconds of trail
pub const TRAIL_MAX_LENGTH: usize = 5;
/// Minimum size of object when rendering circles
pub const MIN_CIRCLE_SIZE: f32 = 0.05;

/// Use barnes-hut if there are more than this many objects
pub const BARNES_HUT_CUTOFF: usize = 1000;
/// Barnes-Hut coefficient (theta). Smaller values = more accurate, but slower.
pub const BARNES_HUT_COEFF: f64 = 0.3;
