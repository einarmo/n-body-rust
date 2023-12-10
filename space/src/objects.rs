use std::ops::Range;

use wgpu::{Buffer, Queue, VertexAttribute, VertexBufferLayout};

pub type Vec3 = [f32; 3];

pub const TRAIL_MAX_LENGTH: usize = 100;
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

#[derive(Clone)]
pub struct ObjectTrailInner(pub [Vertex; TRAIL_MAX_LENGTH]);

impl Default for ObjectTrailInner {
    fn default() -> Self {
        Self([Vertex::default(); TRAIL_MAX_LENGTH])
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
}

impl Default for ObjectInstance {
    fn default() -> Self {
        Self {
            color: [1.0f32, 1.0f32, 1.0f32],
        }
    }
}

impl ObjectInstance {
    pub const fn layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<ObjectInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 2,
            }],
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

    pub fn push_items(&mut self, batch: PointBatch) {
        assert!(batch.len() == self.num_objects);

        for point in batch.into_iter() {
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
        if self.pending_tail > self.pending_head {
            let slice = &self.buff[self.pending_head..self.pending_tail];
            let byte_slice = bytemuck::cast_slice(slice);
            queue.write_buffer(buffer, offset, byte_slice);
        } else if self.pending_tail < self.pending_head {
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
        self.pending_head = self.pending_tail;
    }
}

pub struct Objects {
    vertices: ObjectVertexCache,
    descriptions: Vec<ObjectInstance>,
}

impl Objects {
    pub fn new(num_objects: usize) -> Self {
        Self {
            vertices: ObjectVertexCache::new(num_objects),
            descriptions: vec![Default::default(); num_objects],
        }
    }

    pub fn flush_to_buffer(&mut self, buffer: &Buffer, queue: &Queue) {
        self.vertices.flush_to_buffer(buffer, queue);
    }

    pub fn push_items(&mut self, batch: PointBatch) {
        self.vertices.push_items(batch);
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
}
