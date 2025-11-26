pub mod batch_request;
mod camera;
mod circle_pipeline;
pub mod constants;
mod event_loop;
mod objects;
pub mod parameters;
mod pipeline;
mod render;
mod sim;
mod surface;
pub mod ui;

pub use batch_request::BatchRequest;
use bytemuck::{Pod, Zeroable};
use cgmath::Vector3;
pub use event_loop::{SpaceApp, run_sim_loop_erased};
pub use objects::Objects;
pub use sim::{BarnesHutSim, BruteForceSim, ObjectInfo, SimulationImpl};

#[derive(Debug, Clone)]
pub struct Object {
    pub name: String,
    pub dat: ObjectInfo,
    pub color: Vector3<f32>,
    pub radius: f32,
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C, packed)]
struct ShaderConstants {
    pub width: u32,
    pub height: u32,
    pub time: u32,
    pub total_buffer_size: u32,
    pub start_index: u32,
    pub end_index: u32,
    pub use_relative_position: u32,
    pub min_circle_size: f32,
    pub last_relative_position: [f32; 3],
}
