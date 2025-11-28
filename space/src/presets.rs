use cgmath::{EuclideanSpace, InnerSpace, Point3, Vector3};

use crate::{
    Object, ObjectInfo,
    constants::{AU, G, M0},
    parameters::{
        AbsoluteCoords, RelativeCoords, RelativeOrAbsolute, StandardParams, convert_params,
    },
};

pub fn earth_sun_basic() -> Vec<Object> {
    vec![
        Object {
            name: "sun".to_owned(),
            dat: ObjectInfo {
                pos: (0.0, 0.0, 0.0).into(),
                vel: (0.0, 1e3 / AU, 0.0).into(),
                mass: 333000.0,
            },
            color: (1.0, 1.0, 0.0).into(),
            radius: (696340e3 / AU) as f32,
        },
        Object {
            name: "earth".to_owned(),
            dat: ObjectInfo {
                pos: (1.0, 0.0, 0.0).into(),
                vel: (0.0, (29.8e3 + 1e3) / AU, 0.0).into(),
                mass: 1.0,
            },
            color: (0.0, 0.0, 1.0).into(),
            radius: (6371e3 / AU) as f32,
        },
    ]
}

pub fn earth_sun_mars_params() -> Vec<StandardParams> {
    vec![
        StandardParams {
            name: "sun".to_owned(),
            coordinates: RelativeOrAbsolute::Absolute(AbsoluteCoords {
                pos: [0.0, 0.0, 0.0],
                vel: [0.0, 0.0, 0.0],
            }),
            mass: 333000.0,
            radius: (696340e3 / AU) as f32,
            color: (1.0, 1.0, 0.0).into(),
        },
        StandardParams {
            name: "earth".to_owned(),
            coordinates: RelativeOrAbsolute::Relative(RelativeCoords {
                parent: "sun".to_owned(),
                semi_major_axis: 1.495365477412831E+08 * 1e3,
                eccentricity: 1.639588231990315E-02,
                inclination: 3.670030330713475E-03,
                arg_periapsis: 2.557573855355361E+02,
                long_asc_node: 2.087400227953831E+02,
                true_an: 3.450278328909303E+02,
            }),
            /* coordinates: RelativeOrAbsolute::Absolute(AbsoluteCoords {
                pos: [0.0, 0.0, 0.0],
                vel: [0.0, 0.0, 0.0],
            }), */
            mass: 1.0,
            radius: (6371e3 / AU) as f32,
            color: (0.0, 0.0, 1.0).into(),
        },
        StandardParams {
            name: "moon".to_owned(),
            coordinates: RelativeOrAbsolute::Relative(RelativeCoords {
                parent: "earth".to_owned(),
                semi_major_axis: 3.815880763110870E+05 * 1e3,
                eccentricity: 3.179523012872624E-02,
                inclination: 5.064604179512905E+00,
                arg_periapsis: 3.012277898101174E+02,
                long_asc_node: 2.229402837659016E+01,
                true_an: 6.454243862420770E+01,
            }),
            mass: 7.349e22 / M0,
            radius: (1737e3 / AU) as f32,
            color: (1.0, 1.0, 1.0).into(),
        },
        StandardParams {
            name: "mars".to_owned(),
            coordinates: RelativeOrAbsolute::Relative(RelativeCoords {
                parent: "sun".to_owned(),
                semi_major_axis: 227956E+6,
                eccentricity: 0.0935,
                inclination: 1.848,
                arg_periapsis: 286.5,
                long_asc_node: 49.578,
                true_an: 0.0, // TOOD
            }),
            mass: 0.107,
            radius: (3396.2e3 / AU) as f32,
            color: (1.0, 0.0, 0.0).into(),
        },
    ]
}

#[allow(clippy::excessive_precision)] // Copy-pasted from online sources
pub fn earth_sun_mars() -> Vec<Object> {
    convert_params(earth_sun_mars_params())
        .into_iter()
        .map(|o| o.into())
        .collect()
}

pub fn big_boy_on_collision_course() -> Object {
    Object {
        name: "big_boy".to_owned(),
        dat: ObjectInfo {
            pos: (3.0, 0.0, 0.0).into(),
            vel: (-0.5e5 / AU, -0.2e5 / AU, 0.0).into(),
            mass: 100000.0,
        },
        color: (0.0, 1.0, 0.0).into(),
        radius: (1e6 / AU) as f32,
    }
}

pub fn earth_sun_mars_ast() -> Vec<Object> {
    let mut objs = earth_sun_mars_params();
    objs.append(&mut asteroid_belt(10000));
    convert_params(objs).into_iter().map(|o| o.into()).collect()
}

pub fn asteroid_belt(n_asteroids: usize) -> Vec<StandardParams> {
    let mut objs = Vec::new();
    for i in 0..n_asteroids {
        let col = 0.5 + rand::random_range(-0.2..0.2);
        objs.push(StandardParams {
            name: format!("asteroid_{i}"),
            coordinates: RelativeOrAbsolute::Relative(RelativeCoords {
                parent: "sun".to_owned(),
                semi_major_axis: 300000E+6 + rand::random_range(-1.0..1.0) * 25_000E+6,
                eccentricity: rand::random_range(0.0..0.15),
                inclination: rand::random_range(0.0..10.0),
                arg_periapsis: rand::random_range(0.0..360.0),
                long_asc_node: rand::random_range(0.0..360.0),
                true_an: rand::random_range(0.0..360.0),
            }),
            mass: rand::random_range(1e-10..1e-6),
            radius: rand::random_range((1e3 / AU)..(1e6 / AU)) as f32,
            color: (col, col, col).into(),
        });
    }
    objs
}

pub fn fixed_cloud(n_objects: usize) -> Vec<Object> {
    let min = -10.0;
    let max = 10.0;
    let idx_step = (n_objects as f64).cbrt().ceil() as usize;
    let step = (max - min) / (idx_step as f64);

    let mut objs = Vec::new();
    objs.push(Object {
        name: "Center".to_owned(),
        dat: ObjectInfo {
            pos: Point3::new(-15.0, 0.0, 0.0),
            vel: Vector3::new(0.0, 0.0, 0.0),
            mass: 1e7,
        },
        color: Vector3::new(1.0, 1.0, 1.0),
        radius: (1e5 / AU) as f32,
    });

    for i in 0..n_objects {
        let pos = Point3::new(
            min + (i % idx_step) as f64 * step,
            min + ((i / idx_step) % idx_step) as f64 * step,
            min + ((i / (idx_step * idx_step)) % idx_step) as f64 * step,
        );
        let rotate_around = Vector3::new(0.0, 1.0, 1.0).normalize();
        let radius = (pos - Point3::new(-15.0, 0.0, 0.0)).magnitude();
        let norm_pos = (pos - Point3::new(-15.0, 0.0, 0.0)).normalize();
        // let vel = rotate_around.cross(norm_pos) * (1e6 / AU) * (radius);
        let vel_basis = (G * 1e7 / radius).sqrt();
        let vel = rotate_around.cross(norm_pos) * vel_basis;

        let col = (pos.to_vec() - Vector3::new(min, min, min))
            .normalize()
            .cast()
            .unwrap();
        objs.push(Object {
            name: format!("particle_{i}"),
            dat: ObjectInfo {
                pos: pos,
                vel: vel,
                mass: 1e4,
            },
            color: col,
            radius: (1e4 / AU) as f32,
        });
    }

    objs
}

#[allow(unused)]
fn fixed_shell(n_objects: usize) -> Vec<Object> {
    let idx_step = (n_objects as f64).sqrt().ceil() as usize;
    let pi_step = std::f64::consts::PI / (idx_step as f64);

    let mut objs = Vec::new();
    objs.push(Object {
        name: "Center".to_owned(),
        dat: ObjectInfo {
            pos: Point3::new(0.0, 0.0, 0.0),
            vel: Vector3::new(0.0, 0.0, 0.0),
            mass: 1e7,
        },
        color: Vector3::new(1.0, 1.0, 1.0),
        radius: (1e5 / AU) as f32,
    });
    for i in 0..n_objects {
        let theta = pi_step * ((i / idx_step) % idx_step) as f64;
        let phi = 2.0 * pi_step * (i % idx_step) as f64;
        let radius = 10.0;

        let pos = Point3::new(
            radius * theta.sin() * phi.cos(),
            radius * theta.sin() * phi.sin(),
            radius * theta.cos(),
        );

        let rotate_around = Vector3::new(0.0, 1.0, 1.0).normalize();
        let radius = pos.to_vec().magnitude();
        let norm_pos = pos.to_vec().normalize();
        // let vel = rotate_around.cross(norm_pos) * (1e6 / AU) * (radius);
        let vel_basis = (G * 1e7 / radius).sqrt();
        let vel = rotate_around.cross(norm_pos).normalize() * vel_basis;

        let col = (pos.to_vec() + Vector3::new(radius, radius, radius))
            .normalize()
            .cast()
            .unwrap();
        objs.push(Object {
            name: format!("particle_{i}"),
            dat: ObjectInfo {
                pos: pos,
                vel,
                mass: 0.0,
            },
            color: col,
            radius: (1e4 / AU) as f32,
        });
    }

    objs
}
