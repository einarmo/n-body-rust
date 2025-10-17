use std::time::Instant;

use cgmath::{InnerSpace, Vector3};

use crate::sim::{
    ObjectInfo,
    barnes_hut::tree::{FmmTree, ObjectId},
};

mod tree;

pub fn iter(info: &mut [ObjectInfo], out: &mut [Vector3<f64>], theta: f64) {
    // let mut start = Instant::now();
    let tree = FmmTree::new(info);
    // println!("Tree built in {:?}", start.elapsed());
    // start = Instant::now();
    let theta_sq = theta * theta;

    let mut comp_count = 0;

    for (i, obj) in tree.iter_objects() {
        let out_acc = &mut out[i.to_index()];
        compute_acc(i, &tree, obj, out_acc, theta_sq, &mut comp_count);
    }
    // println!("Acceleration computed in {:?}", start.elapsed());

    /* if tree.len() > 10 {
        println!("{tree:#?}");
    } */
    // panic!("Done");
}

fn compute_acc(
    id: ObjectId,
    tree: &FmmTree<'_>,
    obj: &ObjectInfo,
    out: &mut Vector3<f64>,
    theta_sq: f64,
    comp_count: &mut u64,
) {
    let estimate = 8 * (tree.len() as f32).ln() as usize;
    let mut stack = Vec::with_capacity(estimate);
    stack.push(tree.root_id());

    while let Some(node_id) = stack.pop() {
        let node = tree.get(node_id);

        match &node.data {
            tree::NodeData::External { point } => {
                if point == &id {
                    continue;
                }
                *comp_count += 1;
                obj.get_acc_towards(&tree.get_object(*point), out);
            }
            tree::NodeData::Internal {
                children,
                center_mass,
                mass,
            } => {
                let rel = center_mass - obj.pos;
                let dist_sq = rel.magnitude2();
                if dist_sq == 0.0 {
                    continue;
                }
                let size_sq = node.region.size() * node.region.size();
                if dist_sq * theta_sq < size_sq {
                    stack.extend(children);
                } else {
                    // Treat this node as a single body
                    *comp_count += 1;
                    obj.get_acc_towards_raw(center_mass, *mass, out);
                }
            }
        }
    }
}
