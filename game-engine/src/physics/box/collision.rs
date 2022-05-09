use crate::physics::{
    collision::{pop_coliders, standard_collision},
    sphere::SphereColider,
    Collider, RidgidBody,
};

use super::{
    mesh::{get_rays_for_box, get_tris_for_box, get_verts},
    sat::{get_axis_and_verts, proj_has_overlap},
    BoxColider,
};

use common::{Ray, Transform, Vec3};

pub fn is_colliding_box_vs_box(
    w1: Vec3,
    w2: Vec3,
    bc1: &BoxColider,
    t1: &Transform,
    bc2: &BoxColider,
    t2: &Transform,
) -> bool {
    let (axis, a_verts, b_verts) = get_axis_and_verts(&w1, &w2, &t1, &t2, bc1, bc2);
    proj_has_overlap(&axis, &a_verts, &b_verts) || proj_has_overlap(&axis, &b_verts, &a_verts)
}

pub fn collide_box_vs_box(
    c1: &BoxColider,
    rb1: &mut RidgidBody,
    t1: &mut Transform,
    w1: Vec3, // world position
    c2: &BoxColider,
    rb2: &mut RidgidBody,
    t2: &mut Transform,
    w2: Vec3, // world position
) {
    if rb1.is_static && rb2.is_static {
        return;
    }

    // this ensures that rb2 is never static
    if rb1.is_static {
        return collide_box_vs_box(c2, rb2, t2, w2, c1, rb1, t1, w1);
    }
    let v1 = get_verts(t1, c1);
    let v2 = get_verts(t2, c2);

    let mut rays = get_rays_for_box(&v1);
    let tri2 = get_tris_for_box(&v2);

    // casting rays on the AABB c2 in a cordinate system where w2 is the Origin
    let world_offset = w1 - w2;

    let r1 = t1.rotation * c1.local_rotation;
    let r2 = t2.rotation * c2.local_rotation;

    let re1 = c1.material.restfullness;
    let re2 = c2.material.restfullness;

    //t1.rotation = Quaternion::identity();
    //t2.rotation = Quaternion::identity();

    let r2_inv = r2.inverse();

    let s1 = t1.scale * c1.scale;

    for ray in &mut rays {
        let origin = r2_inv *            // rotate ray
            (r1 *                                  // rotation on self 
                (ray.origin + s1 * ray.direction)  // set ray origin between vertexes, this is used because ray intercect returns negetive values
                                  + world_offset); // applied offset to center the world on w2

        let direction = r2_inv * r1 * ray.direction;
        let new_ray = Ray::new(origin, direction);

        for tri in &tri2 {
            if let Some(d) = new_ray.triangle_intersection(*tri) {
                let ray_distance = d.abs();
                if ray_distance <= f32::EPSILON {
                    continue;
                }

                let box_distance = (ray.direction * s1).magnitude();

                // ray hit is not outside the box
                if ray_distance < box_distance {
                    let overlap = box_distance - ray_distance;
                    let normal = -(r2 * ray.direction).normalized();
                    let normal_overlap = normal * overlap;

                    //pop_coliders(d.signum() * normal_overlap / 4.0, t1, t2, &rb1, &rb2);

                    let point_of_contact =
                        w1 + r2 * (ray.direction * d + ray.origin) - s1 * ray.direction;

                    standard_collision(
                        normal,
                        (rb1, rb2),
                        //(&Collider::BoxColider(*c1), &Collider::BoxColider(*c2)),
                        (&*t1, &*t2),
                        (c1.inv_inertia_tensor(), c2.inv_inertia_tensor()),
                        (point_of_contact - w1, point_of_contact - w2),
                        re1,
                        re2,
                    );
                }
            }
        }
    }
}
