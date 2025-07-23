#![allow(clippy::too_many_arguments)]
#![no_std]
use spirv_std::glam::{Mat4, Vec2, Vec3, Vec4, Vec4Swizzles, vec4};
use spirv_std::image::Image2d;
use spirv_std::num_traits::Float;
use spirv_std::{Sampler, spirv};

#[repr(C)]
pub struct CameraUniform {
    pub view_proj: Mat4,
    pub view: Mat4,
    pub projection: Mat4,
}

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
pub fn line_vs(
    #[spirv(push_constant)] constants: &ShaderConstants,
    input_pos: Vec3,
    input_idx: u32,
    instance_color: Vec3,
    _instance_size: f32,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] camera_uniform: &CameraUniform,
    #[spirv(position, invariant)] out_pos: &mut Vec4,
    out_color: &mut Vec4,
) {
    let index_offset = (input_idx + constants.total_buffer_size - constants.start_index)
        % constants.total_buffer_size;

    let current_vertex_count = (constants.end_index + constants.total_buffer_size
        - constants.start_index)
        % constants.total_buffer_size;

    let floating_offset = index_offset as f32 / current_vertex_count as f32;
    // For some reason, doing the multiplication in two stages is much more stable
    // when zoomed in.
    let pos_view = camera_uniform.view * Vec4::from((input_pos, 1.0));
    *out_pos = camera_uniform.projection * pos_view;
    *out_color = vec4(
        instance_color.x,
        instance_color.y,
        instance_color.z,
        floating_offset,
    );
}

#[spirv(fragment)]
pub fn line_fs(
    in_color: Vec4,
    // #[spirv(push_constant)] _constants: &ShaderConstants,
    output: &mut Vec4,
) {
    //*output = Vec4::new(1.0, 1.0, 1.0, 1.0);
    *output = in_color.xyz().extend(in_color.w);
}

const CLIP_SPACE_COORD_QUAD_CCW: [Vec2; 6] = {
    let tl = Vec2::new(-1.0, 1.0);
    let tr = Vec2::new(1.0, 1.0);
    let bl = Vec2::new(-1.0, -1.0);
    let br = Vec2::new(1.0, -1.0);
    [bl, br, tr, tr, tl, bl]
};

#[spirv(vertex)]
pub fn circle_vs(
    #[spirv(push_constant)] constants: &ShaderConstants,
    #[spirv(vertex_index)] vertex_id: u32,
    input_instance_pos: Vec3,
    _input_idx: u32,
    input_instance_color: Vec3,
    input_instance_size: f32,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] camera_uniform: &CameraUniform,
    #[spirv(position)] out_pos: &mut Vec4,
    out_color: &mut Vec4,
    out_uv: &mut Vec2,
) {
    let index = vertex_id as usize % 6;
    let raw = CLIP_SPACE_COORD_QUAD_CCW[index];
    let raw_shifted = Vec2::new(
        raw.x / (constants.width as f32 / constants.height as f32),
        raw.y,
    );

    let center_view = camera_uniform.view * Vec4::from((input_instance_pos, 1.0));
    let center_proj = camera_uniform.projection * center_view;
    // There is certainly some clever math to avoid this, but I can't be bothered.
    // Use the projection of another point offset from the target to get the size.
    // (|P * (v + s) - P * v| = |P * s|)

    // let pert_view = center_view + Vec4::new(input_instance_size, 0.0, 0.0, 0.0);
    // let pert_proj = camera_uniform.projection * pert_view;

    // let projected_size = (pert_proj - center_proj).xy().length();

    let projected_size = (camera_uniform.projection
        * Vec4::new(input_instance_size, 0.0, 0.0, 1.0))
    .xy()
    .length();

    *out_pos = Vec4::from((
        center_proj.xy() + projected_size * raw_shifted,
        center_proj.z,
        center_proj.w,
    ));

    *out_color = Vec4::from((input_instance_color, 1.0));
    *out_uv = raw;
}

#[spirv(fragment)]
pub fn circle_fs(in_color: Vec4, in_uv: Vec2, out_color: &mut Vec4) {
    let radius = in_uv.length_squared();
    *out_color = in_color;
    out_color.w = (1.0 - Float::powi(radius, 2)).clamp(0.0, 1.0);
}

#[spirv(vertex)]
pub fn copy_texture_vs(
    #[spirv(vertex_index)] vertex_id: u32,
    #[spirv(position)] out_pos: &mut Vec4,
    out_uv: &mut Vec2,
) {
    let index = vertex_id as usize % 6;
    let raw = CLIP_SPACE_COORD_QUAD_CCW[index];
    *out_pos = Vec4::new(raw.x, raw.y, 0.0, 1.0);
    *out_uv = (raw + Vec2::splat(1.0)) / 2.0;
}

#[spirv(fragment)]
pub fn copy_texture_fs(
    in_uv: Vec2,
    #[spirv(descriptor_set = 0, binding = 0)] image: &Image2d,
    #[spirv(descriptor_set = 0, binding = 1)] sampler: &Sampler,
    out_color: &mut Vec4,
) {
    *out_color = image.sample(*sampler, in_uv);
}
