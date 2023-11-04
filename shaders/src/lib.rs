#![cfg_attr(target_arch = "spirv", no_std)]

use spirv_std::glam::{vec4, Vec3, Vec4};
use spirv_std::spirv;

#[repr(C)]
pub struct Vertex {
    pos: Vec3,
}

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(vertex_index)] vert_id: i32,
    input_pos: Vertex,
    #[spirv(position, invariant)] out_pos: &mut Vec4,
) {
    *out_pos = vec4(input_pos.pos.x, input_pos.pos.y, 0.0, 1.0);
}

#[spirv(fragment)]
pub fn main_fs(output: &mut Vec4) {
    *output = vec4(1.0, 0.0, 0.0, 1.0);
}
