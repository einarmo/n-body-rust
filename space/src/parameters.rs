use std::collections::HashMap;

use cgmath::{num_traits::Pow, Angle, Deg, InnerSpace, Point3, Rad, Vector3, Zero};

use crate::{
    sim::{ObjectInfo, AU, G_ABS, M0},
    Object,
};

pub struct ConvertedOrbitalParams {
    name: String,
    index: usize,
    parent_index: Option<usize>,
    pos: Point3<f64>,
    vel: Vector3<f64>,
    color: Vector3<f32>,
    radius: f32,
    mass: f64,
    children_mass: f64,
}

impl From<ConvertedOrbitalParams> for Object {
    fn from(value: ConvertedOrbitalParams) -> Self {
        Self {
            dat: ObjectInfo {
                pos: value.pos / AU,
                vel: value.vel / AU,
                mass: value.mass,
            },
            color: value.color,
            radius: value.radius,
        }
    }
}

#[derive(Debug)]
pub struct AbsoluteCoords {
    pub pos: [f64; 3],
    pub vel: [f64; 3],
}

#[derive(Debug)]
pub struct RelativeCoords {
    pub parent: String,
    // In meters
    pub semi_major_axis: f64,
    // [0, 1]
    pub eccentricity: f64,
    // In degrees
    pub inclination: f64,
    // In degrees
    pub arg_periapsis: f64,
    // In degrees
    pub long_asc_node: f64,
    // In degrees
    pub true_an: f64,
}

pub enum RelativeOrAbsolute {
    Absolute(AbsoluteCoords),
    Relative(RelativeCoords),
}

pub struct StandardParams {
    pub name: Option<String>,
    pub coordinates: RelativeOrAbsolute,
    pub mass: f64,
    pub radius: f32,
    pub color: [f32; 3],
}

fn compute_from_orbital_params(
    parent: &ConvertedOrbitalParams,
    coords: RelativeCoords,
    mass: f64,
) -> AbsoluteCoords {
    let mu = G_ABS * (parent.mass * M0 + mass * M0);
    let true_anom: Rad<f64> = Deg(coords.true_an).into();
    let ecc_anomaly = Rad(f64::atan2(
        (1.0 - coords.eccentricity.pow(2) as f64).sqrt() * true_anom.0.sin(),
        coords.eccentricity + true_anom.0.cos(),
    ));

    let radius = coords.semi_major_axis * (1.0 - coords.eccentricity * ecc_anomaly.cos());
    let angular_momentum =
        (mu * coords.semi_major_axis * (1.0 - coords.eccentricity.pow(2) as f64)).sqrt();
    let l_an: Rad<f64> = Deg(coords.long_asc_node).into();
    let arg_per: Rad<f64> = Deg(coords.arg_periapsis).into();
    let inclination: Rad<f64> = Deg(coords.inclination).into();

    let real_angle = arg_per + true_anom;
    let p_x = radius
        * (l_an.cos() * real_angle.cos() - l_an.sin() * real_angle.sin() * inclination.cos());
    let p_y = radius
        * (l_an.sin() * real_angle.cos() + l_an.cos() * real_angle.sin() * inclination.cos());
    let p_z = radius * inclination.sin() * real_angle.cos();

    let p = coords.semi_major_axis * (1.0 - coords.eccentricity.pow(2));
    let velocity_basis = angular_momentum * coords.eccentricity / (radius * p) * true_anom.sin();

    let v_x = p_x * velocity_basis
        - angular_momentum / radius
            * (l_an.cos() * real_angle.sin() + l_an.sin() * real_angle.cos() * inclination.cos());
    let v_y = p_y * velocity_basis
        - angular_momentum / radius
            * (l_an.sin() * real_angle.sin() - l_an.cos() * real_angle.cos() * inclination.cos());
    let v_z =
        p_z * velocity_basis + angular_momentum / radius * inclination.sin() * real_angle.cos();

    println!("Angular momentum: {}", angular_momentum / radius);
    println!("Radius: {}", radius);
    let v_vec = Vector3::new(v_x, v_y, v_z);
    println!("Direction: {:?}", v_vec.normalize());
    let p_vec = Point3::new(p_x, p_y, p_z);
    println!("Vector to parent: {:?}", (parent.pos - p_vec).normalize());
    println!("Velocity basis: {}", velocity_basis);

    println!(
        "Velocity cross direction: {:?}",
        (parent.pos - p_vec).normalize().cross(v_vec.normalize())
    );

    AbsoluteCoords {
        pos: [p_x + parent.pos.x, p_y + parent.pos.y, p_z + parent.pos.z],
        vel: [v_x + parent.vel.x, v_y + parent.vel.y, v_z + parent.vel.z],
    }
}

pub fn convert_params(
    items: impl IntoIterator<Item = StandardParams>,
) -> Vec<ConvertedOrbitalParams> {
    let mut map = HashMap::new();
    let mut res = Vec::new();

    for (idx, item) in items.into_iter().enumerate() {
        let (absolute_coords, parent_idx) = match item.coordinates {
            RelativeOrAbsolute::Absolute(x) => (x, None),
            RelativeOrAbsolute::Relative(r) => {
                let parent = map.get(&r.parent).unwrap();
                (
                    compute_from_orbital_params(parent, r, item.mass),
                    Some(parent.index),
                )
            }
        };

        println!(
            "object: {:?}, {:?}, {:?}",
            item.name, absolute_coords.pos, absolute_coords.vel
        );
        let params = ConvertedOrbitalParams {
            name: item.name.unwrap_or_else(|| idx.to_string()),
            index: idx,
            parent_index: parent_idx,
            pos: absolute_coords.pos.into(),
            vel: absolute_coords.vel.into(),
            color: item.color.into(),
            radius: item.radius,
            mass: item.mass,
            children_mass: 0.0,
        };

        res.push(params.name.clone());
        map.insert(params.name.clone(), params);
    }

    let mut final_vec = Vec::with_capacity(res.len());
    for name in res {
        final_vec.push(map.remove(&name).unwrap());
    }

    // Modify parents with inverse momentum to compensate, making the barycenter
    // of the system fixed.

    for i in (0..final_vec.len()).rev() {
        // Get own base momentum
        let obj = &final_vec[i];
        if let Some(parent_idx) = obj.parent_index.clone() {
            let parent_vel = final_vec[parent_idx].vel;
            let own_relative_momentum = (obj.vel - parent_vel) * (obj.mass + obj.children_mass);

            let parent_mass = final_vec[parent_idx].mass;
            final_vec[parent_idx].children_mass += obj.mass + obj.children_mass;
            final_vec[parent_idx].vel -= own_relative_momentum / parent_mass;
        }

        // Add inverse of child momentum
    }

    final_vec
}
