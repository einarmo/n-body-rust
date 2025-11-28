use cgmath::{InnerSpace, Vector3};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};

use crate::sim::ObjectInfo;

mod tree;

pub(super) use tree::FmmTree;

pub fn iter(info: &mut [ObjectInfo], out: &mut [Vector3<f64>], tree: &mut FmmTree, theta: f64) {
    tree.clear();
    tree.build_tree(info);
    // Edge-case. The Barnes-Hut algorithm does not register massless particles,
    // which elegantly just means that we skip the computation of attraction _towards_
    // these. If there are no massive particles at all, we can skip the entire
    // acceleration computation.
    if tree.len() == 0 {
        return;
    }
    let theta_sq = theta * theta;

    info.par_iter()
        .zip(out.par_iter_mut())
        .for_each(|(obj, out_acc)| {
            compute_acc(&tree, obj, out_acc, theta_sq);
        });
}

pub fn iter_single_threaded(
    info: &mut [ObjectInfo],
    out: &mut [Vector3<f64>],
    tree: &mut FmmTree,
    theta: f64,
) {
    tree.clear();
    tree.build_tree(info);
    let theta_sq = theta * theta;

    for (obj, out_acc) in info.iter().zip(out.iter_mut()) {
        compute_acc(tree, obj, out_acc, theta_sq);
    }
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
                if theta_sq * dist_sq < region.size_sq() =>
            {
                stack.extend(children);
            }
            _ => {
                // Treat this node as a single body
                obj.get_acc_towards_raw(data.mass, rel, dist_sq, out);
            }
        }
    }
}
