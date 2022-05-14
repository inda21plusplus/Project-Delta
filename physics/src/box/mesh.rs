use common::{Ray, Transform, Vec3};

use crate::BoxCollider;

pub type Tri = [Vec3; 3];

/// given a trangle that is counter clockwise, it will return the normal that is normalized
pub fn get_normal_from_tri(tri: &Tri) -> Vec3 {
    -(tri[1] - tri[2]).cross(tri[0] - tri[2]).normalized()
}

/// get proper vertex position in world position
pub fn get_vertex(w: &Vec3, t: &Transform, c: &BoxCollider) -> Vec<Vec3> {
    let s = c.scale * t.scale;
    let r = t.rotation * c.local_rotation;
    let mut vec: Vec<Vec3> = Vec::with_capacity(8);

    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            for z in [-1.0, 1.0] {
                vec.push(w + r * Vec3::new(s.x * x, s.y * y, s.z * z))
            }
        }
    }

    vec
}

/// in binary order, aka v000 v001 v010, not rotated where v000 is min and v111 is max,
/// note that it does not apply rotation or world position
pub fn get_verts(t: &Transform, c: &BoxCollider) -> [Vec3; 8] {
    let c = t.scale * c.scale;
    let v111 = c;
    let v000 = -c;

    let v001 = Vec3::new(v000.x, v000.y, v111.z);
    let v010 = Vec3::new(v000.x, v111.y, v000.z);
    let v100 = Vec3::new(v111.x, v000.y, v000.z);

    let v011 = Vec3::new(v000.x, v111.y, v111.z);
    let v110 = Vec3::new(v111.x, v111.y, v000.z);
    let v101 = Vec3::new(v111.x, v000.y, v111.z);

    [v000, v001, v010, v011, v100, v101, v110, v111]
}

#[test]
fn test_get_verts() {
    use crate::PhysicsMaterial;

    let scale = Vec3::new(2.0, 1.0, 10.0);

    let t = Transform {
        position: Vec3::zero(),
        rotation: common::Quaternion::identity(),
        scale,
    };

    let material = PhysicsMaterial {
        friction: 1.0,
        restfullness: 1.0,
    };

    let c = BoxCollider::new(Vec3::one(), material);
    let verts = get_verts(&t, &c);

    for x in [-1, 1] {
        for y in [-1, 1] {
            for z in [-1, 1] {
                assert!(verts.contains(&Vec3::new(
                    scale.x * x as f32,
                    scale.y * y as f32,
                    scale.z * z as f32,
                )))
            }
        }
    }
}

pub fn get_rays_for_box(verts: &[Vec3; 8]) -> [Ray; 12] {
    let [v000, v001, v010, v011, v100, v101, v110, _v111] = *verts;
    [
        Ray::new(v000, Vec3::unit_x()),
        Ray::new(v001, Vec3::unit_x()),
        Ray::new(v010, Vec3::unit_x()),
        Ray::new(v011, Vec3::unit_x()),
        Ray::new(v000, Vec3::unit_y()),
        Ray::new(v001, Vec3::unit_y()),
        Ray::new(v100, Vec3::unit_y()),
        Ray::new(v101, Vec3::unit_y()),
        Ray::new(v000, Vec3::unit_z()),
        Ray::new(v010, Vec3::unit_z()),
        Ray::new(v100, Vec3::unit_z()),
        Ray::new(v110, Vec3::unit_z()),
    ]
}

pub fn get_tris_for_box(verts: &[Vec3; 8]) -> [Tri; 12] {
    let [v000, v001, v010, v011, v100, v101, v110, v111] = *verts;

    // counter clockwise
    [
        // x face
        [v101, v100, v110],
        [v101, v110, v111],
        // -x face
        [v000, v001, v011],
        [v000, v011, v010],
        // y face
        [v111, v010, v011],
        [v010, v111, v110],
        // -y face
        [v000, v101, v001],
        [v000, v100, v101],
        // z face
        [v001, v101, v111],
        [v001, v111, v011],
        //-z face
        [v000, v010, v100],
        [v010, v110, v100],
    ]
}
