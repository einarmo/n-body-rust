use cgmath::{EuclideanSpace, Point3};

use crate::sim::ObjectInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId(usize);

impl ObjectId {
    pub fn to_index(&self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

#[derive(Debug, Clone)]
pub struct Region {
    pub x_range: (f64, f64),
    pub y_range: (f64, f64),
    pub z_range: (f64, f64),
}

impl Region {
    pub fn contains(&self, point: &Point3<f64>) -> bool {
        point.x >= self.x_range.0
            && point.x < self.x_range.1
            && point.y >= self.y_range.0
            && point.y < self.y_range.1
            && point.z >= self.z_range.0
            && point.z < self.z_range.1
    }

    pub fn size(&self) -> f64 {
        let x_size = (self.x_range.1 - self.x_range.0).abs();
        x_size
    }
}

#[derive(Debug)]
pub enum NodeData {
    External {
        point: ObjectId,
    },
    Internal {
        children: Vec<NodeId>,
        center_mass: Point3<f64>,
        mass: f64,
    },
}

#[derive(Debug)]
pub struct FmmNode {
    pub data: NodeData,
    pub region: Region,
}

impl FmmNode {
    pub fn new_internal(region: Region, children: Vec<NodeId>) -> Self {
        Self {
            data: NodeData::Internal {
                children,
                center_mass: Point3::new(0.0, 0.0, 0.0),
                mass: 0.0,
            },
            region,
        }
    }

    pub fn new_external(region: Region, point: ObjectId) -> Self {
        Self {
            data: NodeData::External { point },
            region,
        }
    }

    pub fn mass_center_mass(&self, objects: &[ObjectInfo]) -> (Point3<f64>, f64) {
        match &self.data {
            NodeData::External { point } => {
                let obj = &objects[point.0];
                (obj.pos, obj.mass)
            }
            NodeData::Internal {
                center_mass, mass, ..
            } => (*center_mass, *mass),
        }
    }
}

#[derive(Debug)]
pub struct FmmTree<'a> {
    nodes: Vec<FmmNode>,
    pub objects: &'a [ObjectInfo],
}

impl<'a> FmmTree<'a> {
    pub fn new(objects: &'a [ObjectInfo]) -> Self {
        let mut tree = Self {
            nodes: Vec::new(),
            objects,
        };
        tree.build_tree();
        tree
    }

    pub fn iter_objects(&self) -> impl Iterator<Item = (ObjectId, &ObjectInfo)> {
        self.objects
            .iter()
            .enumerate()
            .map(|(i, obj)| (ObjectId(i), obj))
    }

    pub fn root_id(&self) -> NodeId {
        NodeId(0)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn get(&self, node_id: NodeId) -> &FmmNode {
        &self.nodes[node_id.0]
    }

    pub fn get_object(&self, object_id: ObjectId) -> &ObjectInfo {
        &self.objects[object_id.0]
    }

    fn build_tree(&mut self) {
        // Compute the bounding box of all objects
        let mut min = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for obj in self.objects {
            min.x = min.x.min(obj.pos.x) - 0.1;
            min.y = min.y.min(obj.pos.y) - 0.1;
            min.z = min.z.min(obj.pos.z) - 0.1;
            max.x = max.x.max(obj.pos.x) + 0.1;
            max.y = max.y.max(obj.pos.y) + 0.1;
            max.z = max.z.max(obj.pos.z) + 0.1;
        }

        let root = FmmNode::new_internal(
            Region {
                x_range: (min.x, max.x),
                y_range: (min.y, max.y),
                z_range: (min.z, max.z),
            },
            Vec::new(),
        );
        self.nodes.push(root);
        let ids = (0..self.objects.len()).map(ObjectId).collect::<Vec<_>>();
        self.construct_rec(NodeId(0), &ids);
    }

    fn construct_rec(&mut self, node_id: NodeId, points: &[ObjectId]) {
        let node = &self.nodes[node_id.0];
        for octant in IterOctants::new(node.region.clone()) {
            let mut group = Vec::new();
            for point in points {
                let obj = &self.objects[point.0];
                if octant.contains(&obj.pos) {
                    group.push(*point);
                }
            }
            if !group.is_empty() {
                let child_id = NodeId(self.nodes.len());
                if group.len() > 1 {
                    let child_node = FmmNode::new_internal(octant, Vec::new());
                    self.nodes.push(child_node);
                    self.construct_rec(child_id, &group);
                } else {
                    let child_node = FmmNode::new_external(octant, group[0]);
                    self.nodes.push(child_node);
                }
                let node = &mut self.nodes[node_id.0];
                match &mut node.data {
                    NodeData::Internal { children, .. } => children,
                    _ => panic!("Trying to add child to external node"),
                }
                .push(child_id);
            }
        }
        let node = &self.nodes[node_id.0];
        match &node.data {
            NodeData::Internal { children, .. } => {
                // Update center of mass
                let mut center_mass = Point3::new(0.0, 0.0, 0.0);
                let mut total_mass = 0.0;
                for &child_id in children.iter() {
                    let child = &self.nodes[child_id.0];
                    let (child_cm, child_mass) = child.mass_center_mass(&self.objects);
                    center_mass += child_cm.to_vec() * child_mass;
                    total_mass += child_mass;
                }
                center_mass /= total_mass;

                match &mut self.nodes[node_id.0].data {
                    NodeData::Internal {
                        center_mass: cm,
                        mass,
                        ..
                    } => {
                        *cm = center_mass;
                        *mass = total_mass;
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    }
}

struct IterOctants {
    parent: Region,
    x_half: f64,
    y_half: f64,
    z_half: f64,
    index: u8,
}

impl IterOctants {
    pub fn new(parent: Region) -> Self {
        let x_half = (parent.x_range.1 - parent.x_range.0) / 2.0;
        let y_half = (parent.y_range.1 - parent.y_range.0) / 2.0;
        let z_half = (parent.z_range.1 - parent.z_range.0) / 2.0;
        Self {
            parent,
            x_half,
            y_half,
            z_half,
            index: 0,
        }
    }
}

impl Iterator for IterOctants {
    type Item = Region;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= 8 {
            return None;
        }
        let x_0 = if (self.index & 0b100) == 0 {
            self.parent.x_range.0
        } else {
            self.parent.x_range.0 + self.x_half
        };
        let y_0 = if (self.index & 0b010) == 0 {
            self.parent.y_range.0
        } else {
            self.parent.y_range.0 + self.y_half
        };
        let z_0 = if (self.index & 0b001) == 0 {
            self.parent.z_range.0
        } else {
            self.parent.z_range.0 + self.z_half
        };
        let res = Region {
            x_range: (x_0, x_0 + self.x_half),
            y_range: (y_0, y_0 + self.y_half),
            z_range: (z_0, z_0 + self.z_half),
        };
        self.index += 1;
        Some(res)
    }
}
