use common::{Transform, Vec3};

use super::mesh::get_vertex;
use crate::{overlap, BoxColider};

/// SAT algo on 3d
/// https://hitokageproduction.com/article/11
/// https://github.com/irixapps/Unity-Separating-Axis-SAT/
/// https://youtu.be/7Ik2vowGcU0
/// The seperated axis theorem tldr:
/// If 2 shapes colide then all the shadows along all the axis must overlap
#[must_use]
pub fn proj_has_overlap(axis: &Vec<Vec3>, a_verts: &Vec<Vec3>, b_verts: &Vec<Vec3>) -> bool {
    for normal in axis {
        if *normal == Vec3::zero() {
            return true;
        }
        let (a_min, a_max) = get_min_max_vert(*normal, a_verts);
        let (b_min, b_max) = get_min_max_vert(*normal, b_verts);
        let overlap = overlap(a_min, a_max, b_min, b_max).abs();

        if overlap <= 0.0 {
            return false;
        }
    }

    true
}

/// same as proj_has_overlap with more return info, overlap distance & penetration vector
/// note that penetration is not always normalized
#[must_use]
pub fn proj_has_overlap_extra(
    axis: &Vec<Vec3>,
    a_verts: &Vec<Vec3>,
    b_verts: &Vec<Vec3>,
) -> Option<(f32, Vec3)> {
    let mut min_overlap = f32::INFINITY;
    let mut penetration = Vec3::zero();
    for normal in axis {
        if *normal == Vec3::zero() {
            return Some((min_overlap, penetration));
        }
        let (a_min, a_max) = get_min_max_vert(*normal, a_verts);
        let (b_min, b_max) = get_min_max_vert(*normal, b_verts);
        let overlap = overlap(a_min, a_max, b_min, b_max);
        let abs_overlap = overlap.abs();
        if abs_overlap <= 0.0 {
            return None;
        }

        if abs_overlap < min_overlap.abs() {
            min_overlap = overlap;
            penetration = *normal;
        }
    }

    Some((min_overlap, penetration))
}

fn get_min_max_vert(normal: Vec3, verts: &Vec<Vec3>) -> (f32, f32) {
    let mut proj_min = f32::MAX;
    let mut proj_max = f32::MIN;
    for vert in verts {
        let val = vert.dot(normal);
        if val < proj_min {
            proj_min = val;
        }

        if val > proj_max {
            proj_max = val;
        }
    }
    (proj_min, proj_max)
}

/// returns (1,0,0) (0,1,0) (0,0,1) with rotation aka positive normals
pub fn get_axis(t: &Transform, c: &BoxColider) -> (Vec3, Vec3, Vec3) {
    let rotation = t.rotation * c.local_rotation;
    (
        rotation * Vec3::unit_x(),
        rotation * Vec3::unit_y(),
        rotation * Vec3::unit_z(),
    )
}

/// returns all the axis for SAT to use
#[must_use]
pub fn get_axis_and_verts(
    w1: &Vec3,
    w2: &Vec3,
    t1: &Transform,
    t2: &Transform,
    bc1: &BoxColider,
    bc2: &BoxColider,
) -> (Vec<Vec3>, Vec<Vec3>, Vec<Vec3>) {
    let (a0, a1, a2) = get_axis(&t1, bc1);
    let (b0, b1, b2) = get_axis(&t2, bc2);

    let axis = vec![
        a0,
        a1,
        a2,
        b0,
        b1,
        b2,
        a0.cross(b0),
        a0.cross(b1),
        a0.cross(b2),
        a1.cross(b0),
        a1.cross(b1),
        a1.cross(b2),
        a2.cross(b0),
        a2.cross(b1),
        a2.cross(b2),
    ];

    let a_vex = get_vertex(w1, &t1, bc1);
    let b_vex = get_vertex(w2, &t2, bc2);
    (axis, a_vex, b_vex)
}
