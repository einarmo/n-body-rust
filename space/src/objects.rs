use std::ops::Range;

use wgpu::{Buffer, Queue, VertexAttribute, VertexBufferLayout};

use crate::Object;

pub type Vec3 = [f32; 3];

// 2 minutes of trail
pub const TRAIL_MAX_LENGTH: usize = 60 * 60 * 2;
pub const OBJECT_STRIDE: usize = TRAIL_MAX_LENGTH * std::mem::size_of::<Vertex>();

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: Vec3,
    pub idx: u32,
}

impl Vertex {
    pub const fn layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute {
                    format: wgpu::VertexFormat::Uint32,
                    offset: 3 * std::mem::size_of::<f32>() as u64,
                    shader_location: 1,
                },
            ],
        }
    }
}

pub struct ObjectVertexCache {
    buff: Vec<Vertex>,
    num_objects: usize,
    head: usize,
    tail: usize,
    pending_head: usize,
    pending_tail: usize,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ObjectInstance {
    pub color: [f32; 3],
    pub position: [f32; 3],
    pub radius: f32,
}

impl ObjectInstance {
    pub const fn layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<ObjectInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 2,
                },
                VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: (std::mem::size_of::<f32>() * 3) as u64,
                    shader_location: 3,
                },
                VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: (std::mem::size_of::<f32>() * 6) as u64,
                    shader_location: 4,
                },
            ],
        }
    }
}

pub type PointBatch<'a> = &'a [Vec3];

impl ObjectVertexCache {
    pub fn new(num_objects: usize) -> Self {
        Self {
            buff: vec![Default::default(); num_objects * TRAIL_MAX_LENGTH],
            num_objects,
            head: 0,
            tail: 0,
            pending_head: 0,
            pending_tail: 0,
        }
    }

    fn inc_circular(head: &mut usize, tail: &mut usize, len: usize) {
        *tail = (*tail + 1) % len;
        if *tail == *head {
            *head = (*head + 1) % len;
        }
    }

    pub fn push_items(&mut self, batch: &PointBatch) {
        debug_assert!(batch.len() == self.num_objects);

        for point in batch.iter() {
            self.buff[self.pending_tail] = Vertex {
                pos: *point,
                idx: self.tail as u32,
            };

            Self::inc_circular(
                &mut self.pending_head,
                &mut self.pending_tail,
                TRAIL_MAX_LENGTH * self.num_objects,
            );
        }

        Self::inc_circular(&mut self.head, &mut self.tail, TRAIL_MAX_LENGTH);
    }

    pub fn flush_to_buffer(&mut self, buffer: &Buffer, queue: &Queue) {
        let offset = (self.pending_head * std::mem::size_of::<Vertex>()) as u64;
        match self.pending_tail.cmp(&self.pending_head) {
            // Buffer is wrapping around
            std::cmp::Ordering::Less => {
                queue.write_buffer(
                    buffer,
                    offset,
                    bytemuck::cast_slice(&self.buff[self.pending_head..]),
                );
                queue.write_buffer(
                    buffer,
                    0,
                    bytemuck::cast_slice(&self.buff[0..self.pending_tail]),
                );
            }
            // Buffer is empty
            std::cmp::Ordering::Equal => (),
            // Buffer is not wrapping
            std::cmp::Ordering::Greater => {
                let slice = &self.buff[self.pending_head..self.pending_tail];
                let byte_slice = bytemuck::cast_slice(slice);
                queue.write_buffer(buffer, offset, byte_slice);
            }
        }
        self.pending_head = self.pending_tail;
    }

    pub fn position_of(&self, idx: usize) -> &[f32; 3] {
        let mut vertex_idx_raw = idx as i64 - self.num_objects as i64 + self.pending_tail as i64;
        if vertex_idx_raw < 0 {
            vertex_idx_raw = TRAIL_MAX_LENGTH as i64 * self.num_objects as i64 - vertex_idx_raw;
        }
        &self.buff[vertex_idx_raw as usize].pos
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.pending_head = 0;
        self.pending_tail = 0;
    }
}

pub struct Objects {
    vertices: ObjectVertexCache,
    descriptions: Vec<ObjectInstance>,
}

impl Objects {
    pub fn new(init: &[Object]) -> Self {
        let num_objects = init.len();
        let mut descriptions = Vec::with_capacity(num_objects);
        for obj in init {
            descriptions.push(ObjectInstance {
                color: obj.color.into(),
                position: {
                    let x: [f64; 3] = obj.dat.pos.into();
                    x.map(|f| f as f32)
                },
                radius: obj.radius,
            })
        }

        Self {
            vertices: ObjectVertexCache::new(num_objects),
            descriptions,
        }
    }

    pub fn flush_to_buffer(&mut self, buffer: &Buffer, queue: &Queue) {
        self.vertices.flush_to_buffer(buffer, queue);
    }

    pub fn push_items(&mut self, batch: PointBatch) {
        self.vertices.push_items(&batch);
        for (idx, pos) in batch.iter().enumerate() {
            self.descriptions[idx].position = *pos;
        }
    }

    pub fn get_index_range(&self) -> Range<u32> {
        let head = self.vertices.head as u32;
        if self.vertices.tail >= self.vertices.head {
            head..(head + self.vertices.tail as u32)
        } else {
            head..((TRAIL_MAX_LENGTH + self.vertices.tail) as u32)
        }
    }

    pub fn num_objects(&self) -> usize {
        self.descriptions.len()
    }

    pub fn descriptions_mut(&mut self) -> &mut [ObjectInstance] {
        self.descriptions.as_mut_slice()
    }

    pub fn position_of(&self, idx: usize) -> &[f32; 3] {
        let idx = idx % self.num_objects();
        self.vertices.position_of(idx)
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
    }
}
