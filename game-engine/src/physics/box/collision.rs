use crate::physics::{
    collision::standard_collision,
    macros::{debug_assert_finite, debug_assert_normalized},
    RayCastHit, RidgidBody, Tri,
};

use super::{
    mesh::{get_rays_for_box, get_tris_for_box, get_verts},
    sat::{get_axis_and_verts, proj_has_overlap, proj_has_overlap_extra},
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
    if rb2.is_static {
        collide_box_vs_box_single(c1, rb1, t1, w1, c2, rb2, t2, w2);
    } else {
        collide_box_vs_box_single(c2, rb2, t2, w2, c1, rb1, t1, w1);
    }
}

/// given a trangle that is counter clockwise, it will return the normal that is normalized
fn get_normal_from_tri(tri: &Tri) -> Vec3 {
    return -(tri[1] - tri[2]).cross(tri[0] - tri[2]).normalized();
}

fn collide_box_vs_box_single(
    c1: &BoxColider,
    rb1: &mut RidgidBody,
    t1: &mut Transform,
    w1: Vec3, // world position
    c2: &BoxColider,
    rb2: &mut RidgidBody,
    t2: &mut Transform,
    w2: Vec3, // world position
) {
    let v1 = get_verts(t1, c1);
    let v2 = get_verts(t2, c2);

    let mut rays = get_rays_for_box(&v1);
    let tri2 = get_tris_for_box(&v2);

    // casting rays on the AABB c2 in a cordinate system where w2 is the Origin
    let world_offset = w1 - w2;

    let r1 = t1.rotation * c1.local_rotation;
    let r2 = t2.rotation * c2.local_rotation;

    let r2_inv = r2.inverse();
    let s1 = t1.scale * c1.scale;

    for ray in &mut rays {
        let origin = r2_inv *           // rotate ray
            (r1 *                                  // rotation on self 
                (ray.origin + s1 * ray.direction)  // set ray origin between vertexes, this is used because ray intercect returns negetive values
                                  + world_offset); // applied offset to center the world on w2


        let direction = r2_inv * r1 * ray.direction;
        let new_ray = Ray::new(origin, direction);

        for tri in &tri2 {
            let max_distance_on_axis = s1.dot(ray.direction);

            if let Some(d) = new_ray.triangle_intersection(*tri) {
                let ray_distance = d.abs();
                if ray_distance <= f32::EPSILON {
                    continue;
                }
                //direction is norqemalized debug_assert!(direction.is_normalized(), "Direction is not notmalized d = {} |d| = {}",direction, direction.magnitude());
                let box_distance = max_distance_on_axis.abs();

                // ray hit is not outside the box
                if ray_distance < box_distance {
                    let normal = get_normal_from_tri(tri);
                    let point_of_contact = origin + direction * d;
                    
        // to optimize we dont rotate all tris instead we rotate the ray, this is used to get back into world position
        let rotate_right = |world_position: Vec3| -> Vec3 { r2 * world_position + w2 };
        
                    standard_collision(
                        r2 * normal,
                        (rb2, rb1),
                        (&*t2, &*t1),
                        (c2.inv_inertia_tensor(), c1.inv_inertia_tensor()),
                        (
                            rotate_right(point_of_contact) - w2,
                            rotate_right(point_of_contact) - w1,
                        ),
                        (&c1.material, &c2.material),
                    );
                }
            }
        }
    }

    let mut post_offset = Vec3::zero();

    // push the boxes away from each other
    let (axis, a_verts, b_verts) = get_axis_and_verts(&w1, &w2, &t1, &t2, c1, c2);
    if let Some((size, dir)) = proj_has_overlap_extra(&axis, &a_verts, &b_verts) {
        post_offset -= dir.normalized() * size;
    } else if let Some((size, dir)) = proj_has_overlap_extra(&axis, &b_verts, &a_verts) {
        post_offset -= dir.normalized() * size;
    }

    t1.position += post_offset;
}

pub fn raycast_box(t: &Transform, c: &BoxColider, ray: Ray) -> Option<RayCastHit> {
    let v = get_verts(t, c);
    let tris = get_tris_for_box(&v);
    let r = t.rotation * c.local_rotation;
    let r_inv = r.inverse();
    let fixed_ray = Ray::new(r_inv * ray.origin, r_inv * ray.direction);
    let mut min_d = f32::INFINITY;
    let mut normal = Vec3::zero();
    for tri in tris {
        if let Some(d) = fixed_ray.triangle_intersection(tri) {
            if d < std::f32::EPSILON {
                continue;
            }

            if d < min_d {
                min_d = d;
                normal = get_normal_from_tri(&tri);
            }
        }
    }
    if min_d < f32::INFINITY {
        debug_assert_finite!(normal);
        debug_assert_normalized!(normal);
        Some(RayCastHit::new(min_d, r * normal))
    } else {
        None
    }
}
