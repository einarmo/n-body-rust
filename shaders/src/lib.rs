#![cfg_attr(target_arch = "spirv", no_std)]

use spirv_std::glam::{vec4, Vec3, Vec4};
use spirv_std::spirv;

#[repr(C)]
pub struct ShaderConstants {
    pub width: u32,
    pub height: u32,
    pub time: u32,
    pub total_buffer_size: u32,
    pub start_index: u32,
    pub end_index: u32,
}

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(push_constant)] constants: &ShaderConstants,
    input_pos: Vec3,
    input_idx: u32,
    instance_color: Vec3,
    #[spirv(position, invariant)] out_pos: &mut Vec4,
    out_color: &mut Vec4,
) {
    let index_offset = (input_idx + constants.total_buffer_size - constants.start_index)
        % constants.total_buffer_size;

    let current_vertex_count = (constants.end_index + constants.total_buffer_size
        - constants.start_index)
        % constants.total_buffer_size;

    let floating_offset = index_offset as f32 / current_vertex_count as f32;
    *out_pos = vec4(input_pos.x, input_pos.y, 1.0, 1.0);
    *out_color = vec4(
        instance_color.x,
        instance_color.y,
        instance_color.z,
        floating_offset,
    );
}

#[spirv(fragment)]
pub fn main_fs(
    color: Vec4,
    #[spirv(push_constant)] _constants: &ShaderConstants,
    output: &mut Vec4,
) {
    *output = color;
    // *output = vec4(1.0, 0.0, 0.0, opacity);
}
