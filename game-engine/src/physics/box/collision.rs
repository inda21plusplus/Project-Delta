use crate::physics::{
    collision::{pop_coliders, standard_collision},
    sphere::SphereColider,
    Collider, RidgidBody, macros::debug_assert_normalized,
};

use super::{
    mesh::{get_rays_for_box, get_tris_for_box, get_verts},
    sat::{get_axis_and_verts, proj_has_overlap, proj_has_overlap_extra},
    BoxColider,
};

use common::{Ray, Transform, Vec3};
use rendering::Line;

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
    //if rb1.is_static && rb2.is_static {
    //   return;
    //}

    // this ensures that rb2 is never static
    //if rb1.is_static {
    //return collide_box_vs_box(c2, rb2, t2, w2, c1, rb1, t1, w1);
    //}

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
    crate::physics::collision::clear_lines();
    let s1 = t1.scale * c1.scale;
    let mut index = 0;
    let mut post_offset = Vec3::zero();

    let (axis, a_verts, b_verts) = get_axis_and_verts(&w1, &w2, &t1, &t2, c1, c2);
    if let Some((size, dir)) = proj_has_overlap_extra(&axis, &a_verts, &b_verts) {
        post_offset -= dir.normalized() * size ;
    } else if let Some((size, dir)) = proj_has_overlap_extra(&axis, &b_verts, &a_verts) {
        post_offset -= dir.normalized() * size;
    }

    for ray in &mut rays {
        let origin = r2_inv *           // rotate ray
            (r1 *                                  // rotation on self 
                (ray.origin + s1 * ray.direction)  // set ray origin between vertexes, this is used because ray intercect returns negetive values
                                  + world_offset); // applied offset to center the world on w2

        let rotate_right = |world_position: Vec3| -> Vec3 { r2 * world_position + w2 };

        let direction = r2_inv * r1 * ray.direction;
        let new_ray = Ray::new(origin, direction);

        for tri in &tri2 {
            index += 1;
            let max_distance_on_axis = s1.dot(ray.direction);
            crate::physics::collision::set_line_key(
                format!("{} Col2 {}", rb1.id, index),
                Line {
                    start: rotate_right(origin - direction * max_distance_on_axis),
                    end: rotate_right(origin + direction * max_distance_on_axis),
                    color: Vec3::new(1.0, 1.0, 1.0),
                },
            );

            crate::physics::collision::set_line_key(
                format!("{} T1 {}", rb1.id, index),
                Line {
                    start: rotate_right(tri[0]),
                    end: rotate_right(tri[1]),
                    color: Vec3::new(0.0, 1.0, 1.0),
                },
            );
            crate::physics::collision::set_line_key(
                format!("{} T2 {}", rb1.id, index),
                Line {
                    start: rotate_right(tri[1]),
                    end: rotate_right(tri[2]),
                    color: Vec3::new(0.0, 1.0, 1.0),
                },
            );
            crate::physics::collision::set_line_key(
                format!("{} T3 {}", rb1.id, index),
                Line {
                    start: rotate_right(tri[2]),
                    end: rotate_right(tri[1]),
                    color: Vec3::new(0.0, 1.0, 1.0),
                },
            );

            if let Some(d) = new_ray.triangle_intersection(*tri) {
                let ray_distance = d.abs();
                if ray_distance <= f32::EPSILON {
                    continue;
                }
                //direction is norqemalized debug_assert!(direction.is_normalized(), "Direction is not notmalized d = {} |d| = {}",direction, direction.magnitude());
                let box_distance = max_distance_on_axis.abs();

                // ray hit is not outside the box
                if ray_distance < box_distance {
                    let overlap = box_distance - ray_distance;
                    let normal = -(tri[1] - tri[2]).cross(tri[0] - tri[2]).normalized();
                    //let normal = -(r2 * ray.direction).normalized();
                    //let normal_overlap = normal * overlap;

                    // pop_coliders(d.signum() * normal_overlap / 4.0, t1, t2, &rb1, &rb2);
                    //println!("{}/{} -> {}",d,box_distance,normal);
                    let point_of_contact = origin + direction * d;
                    //w1 + //r1*ray.origin  +r1*ray.direction*(d+s1); //+ s1 *r2 * ray.direction;
                    /*crate::physics::collision::set_line_key(
                        format!("{} ColT1 {}",rb1.id,index),
                        Line {
                            start: point_of_contact,
                            end: point_of_contact + Vec3::unit_x(),
                            color: Vec3::new(1.0, 0.0, 0.0),
                        },
                    );
                    crate::physics::collision::set_line_key(
                        format!("{} ColT2 {}",rb1.id,index),
                        Line {
                            start: point_of_contact,
                            end: point_of_contact + Vec3::unit_y(),
                            color: Vec3::new(0.0, 1.0, 0.0),
                        },
                    );
                    crate::physics::collision::set_line_key(
                        format!("{} ColT3 {}",rb1.id,index),
                        Line {
                            start: point_of_contact,
                            end: point_of_contact + Vec3::unit_z(),
                            color: Vec3::new(0.0, 1.0, 0.0),
                        },
                    );*/
                    crate::physics::collision::set_line_key(
                        format!("{} ColT3L {}", rb1.id, index),
                        Line {
                            start: rotate_right(point_of_contact),
                            end: rotate_right(point_of_contact) + r2 * normal,
                            color: Vec3::new(1.0, 0.0, 0.0),
                        },
                    );

                    crate::physics::collision::set_line_key(
                        format!("{} ColT3 {}", rb1.id, index),
                        Line {
                            start: rotate_right(origin),
                            end: rotate_right(point_of_contact),
                            color: Vec3::new(1.0, 0.0, 1.0),
                        },
                    );
                    /*standard_collision(
                        normal,
                        (rb1, rb2),
                        //(&Collider::BoxColider(*c1), &Collider::BoxColider(*c2)),
                        (&*t1, &*t2),
                        (c1.inv_inertia_tensor(), c2.inv_inertia_tensor()),
                        (point_of_contact-w1, point_of_contact-w2),
                        re1,
                        re2,
                    );*/
                    standard_collision(
                        r2 * normal,
                        (rb2, rb1),
                        //(&Collider::BoxColider(*c1), &Collider::BoxColider(*c2)),
                        (&*t2, &*t1),
                        (c2.inv_inertia_tensor(), c1.inv_inertia_tensor()),
                        (
                            rotate_right(point_of_contact) - w2,
                            rotate_right(point_of_contact) - w1,
                        ),
                        re2,
                        re1,
                    );
                    //post_offset += r1*normal*(box_distance-d.abs())/100.0;
                    // break 'outer;
                    // rb1.linear_momentum = Vec3::zero();
                    //rb1.angular_momentum = Vec3::zero();
                }
            }
        }
    }
    t1.position += post_offset;
}
