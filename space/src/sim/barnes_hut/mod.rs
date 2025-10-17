use cgmath::{InnerSpace, Vector3};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};

use crate::sim::{ObjectInfo, barnes_hut::tree::FmmTree};

mod tree;

pub fn iter(info: &mut [ObjectInfo], out: &mut [Vector3<f64>], theta: f64) {
    // let mut start = Instant::now();
    let tree = FmmTree::new(info);
    // println!("Tree built in {:?}", start.elapsed());
    // start = Instant::now();
    let theta_sq = theta * theta;

    info.par_iter()
        .zip(out.par_iter_mut())
        .for_each(|(obj, out_acc)| {
            compute_acc(&tree, obj, out_acc, theta_sq);
        });
    // println!("Acceleration computed in {:?}", start.elapsed());

    /* if tree.len() > 10 {
        println!("{tree:#?}");
    } */
    // panic!("Done");
}

fn compute_acc(tree: &FmmTree, obj: &ObjectInfo, out: &mut Vector3<f64>, theta_sq: f64) {
    let estimate = 8 * (tree.len() as f32).ln() as usize;
    let mut stack = Vec::with_capacity(estimate);
    stack.push(Some(tree.root_id()));

    while let Some(node_id) = stack.pop() {
        let Some(id) = node_id else {
            continue;
        };

        let (node, data) = tree.get(id);

        let rel = data.center_mass - obj.pos;
        let dist_sq = rel.magnitude2();
        if dist_sq == 0.0 {
            continue;
        }

        match &node.data {
            tree::NodeData::Internal { children, region }
                if theta_sq * dist_sq < region.size() * region.size() =>
            {
                stack.extend(children);
            }
            _ => {
                // Treat this node as a single body
                obj.get_acc_towards_raw(&data.center_mass, data.mass, out);
            }
        }
    }
}
