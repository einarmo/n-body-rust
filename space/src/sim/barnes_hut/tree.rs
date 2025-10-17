use cgmath::{EuclideanSpace, Point3};

use crate::sim::ObjectInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

#[derive(Debug, Clone)]
pub struct Region {
    pub x_range: (f64, f64),
    pub y_range: (f64, f64),
    pub z_range: (f64, f64),
}

impl Region {
    pub fn zero() -> Self {
        Self {
            x_range: (0.0, 0.0),
            y_range: (0.0, 0.0),
            z_range: (0.0, 0.0),
        }
    }

    pub fn size(&self) -> f64 {
        let x_size = (self.x_range.1 - self.x_range.0).abs();
        x_size
    }

    pub fn center(&self) -> Point3<f64> {
        Point3::new(
            (self.x_range.0 + self.x_range.1) / 2.0,
            (self.y_range.0 + self.y_range.1) / 2.0,
            (self.z_range.0 + self.z_range.1) / 2.0,
        )
    }
}

#[derive(Debug)]
pub enum NodeData {
    External,
    Internal {
        children: [Option<NodeId>; 8],
        region: Region,
    },
}

#[derive(Debug)]
pub struct FmmNode {
    pub data: NodeData,
}

impl FmmNode {
    pub fn new_internal(region: Region, children: [Option<NodeId>; 8]) -> Self {
        Self {
            data: NodeData::Internal { children, region },
        }
    }

    pub fn new_external() -> Self {
        Self {
            data: NodeData::External,
        }
    }
}

#[derive(Debug)]
pub struct FmmTree {
    nodes: Vec<FmmNode>,
    data: Vec<Data>,
}

#[derive(Debug, Clone)]
pub struct Data {
    pub center_mass: Point3<f64>,
    pub mass: f64,
}

impl FmmTree {
    pub fn new(objects: &[ObjectInfo]) -> Self {
        let mut tree = Self {
            nodes: Vec::new(),
            data: Vec::new(),
        };
        tree.build_tree(objects);
        tree
    }

    pub fn root_id(&self) -> NodeId {
        NodeId(0)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn get(&self, id: NodeId) -> (&FmmNode, &Data) {
        (&self.nodes[id.0], &self.data[id.0])
    }

    fn build_tree(&mut self, objects: &[ObjectInfo]) {
        // Compute the bounding box of all objects
        let mut min = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for obj in objects {
            min.x = min.x.min(obj.pos.x);
            min.y = min.y.min(obj.pos.y);
            min.z = min.z.min(obj.pos.z);
            max.x = max.x.max(obj.pos.x);
            max.y = max.y.max(obj.pos.y);
            max.z = max.z.max(obj.pos.z);
        }

        let data = objects
            .iter()
            .map(|obj| Data {
                center_mass: obj.pos,
                mass: obj.mass,
            })
            .collect::<Vec<_>>();
        self.build_node(
            &data,
            Region {
                x_range: (min.x, max.x),
                y_range: (min.y, max.y),
                z_range: (min.z, max.z),
            },
        );
    }

    fn build_node(&mut self, input: &[Data], region: Region) -> Option<NodeId> {
        if input.is_empty() {
            return None;
        }

        let id = self.nodes.len();
        self.nodes.push(FmmNode::new_external());
        self.data.push(Self::get_data(input));

        if input
            .windows(2)
            .any(|w| w[0].center_mass != w[1].center_mass)
        {
            let center = region.center();
            let mut result = octants(&region).map(|r| (Vec::new(), r));

            for o in input {
                let position = &o.center_mass;
                let index = (0..3).fold(0, |index, i| {
                    index + (usize::from(position[i] < center[i]) << i)
                });
                result[index].0.push(o.clone());
            }

            self.nodes[id] = FmmNode::new_internal(
                region,
                result.map(|(data, region)| self.build_node(&data, region)),
            );
        }

        Some(NodeId(id))
    }

    fn get_data(input: &[Data]) -> Data {
        let mut center_mass = Point3::new(0.0, 0.0, 0.0);
        let mut total_mass = 0.0;
        for obj in input {
            center_mass += obj.center_mass.to_vec() * obj.mass;
            total_mass += obj.mass;
        }
        center_mass /= total_mass;
        Data {
            center_mass,
            mass: total_mass,
        }
    }
}

fn octants(parent: &Region) -> [Region; 8] {
    let mut result = std::array::from_fn(|_| Region::zero());
    let center = parent.center();
    for i in 0..8 {
        let (x_min, x_max) = if (i & 0b001) != 0 {
            (parent.x_range.0, center.x)
        } else {
            (center.x, parent.x_range.1)
        };
        let (y_min, y_max) = if (i & 0b010) != 0 {
            (parent.y_range.0, center.y)
        } else {
            (center.y, parent.y_range.1)
        };
        let (z_min, z_max) = if (i & 0b100) != 0 {
            (parent.z_range.0, center.z)
        } else {
            (center.z, parent.z_range.1)
        };
        result[i] = Region {
            x_range: (x_min, x_max),
            y_range: (y_min, y_max),
            z_range: (z_min, z_max),
        };
    }
    result
}
