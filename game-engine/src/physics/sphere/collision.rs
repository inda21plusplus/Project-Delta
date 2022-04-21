use crate::{physics::{Vec3, r#box::{BoxColider, get_closest_point}, macros::{squared, debug_assert_finite}, RidgidBody, collision::{pop_coliders, standard_collision}}, renderer::Transform};

use super::SphereColider;

pub fn is_colliding_sphere_vs_sphere(
    w1: Vec3,
    w2: Vec3,
    sc1: &SphereColider,
    t1: &Transform,
    sc2: &SphereColider,
    t2: &Transform,
) -> bool {
    let r1 = sc1.get_radius(t1.scale);
    let r2 = sc2.get_radius(t2.scale);

    debug_assert!(r1 > 0.0, "r1 = {}", r1);
    debug_assert!(r2 > 0.0, "r2 = {}", r2);

    let total_radius = r1 + r2;

    w1.distance_squared(w2) <= total_radius * total_radius
}

pub fn is_colliding_sphere_vs_box(
    w1: Vec3,
    w2: Vec3,
    sc1: &SphereColider,
    t1: &Transform,
    bc2: &BoxColider,
    t2: &Transform,
) -> bool {
    let r_squared = squared!(sc1.get_radius(t1.scale));
    debug_assert!(r_squared > 0.0, "r^2 = {}", r_squared);

    let scale = t2.scale * bc2.scale;
    debug_assert!(scale.are_all_positive(), "Scale is negative");
    debug_assert_finite!(scale);

    let closest_point = get_closest_point(w1, w2, scale, t2.rotation);
    closest_point.distance_squared(w1) < r_squared
}

pub fn collide_sphere_vs_sphere(
    c1: &SphereColider,
    rb1: &mut RidgidBody,
    t1: &mut Transform,
    w1: Vec3, // world position
    c2: &SphereColider,
    rb2: &mut RidgidBody,
    t2: &mut Transform,
    w2: Vec3, // world position
) {
    let re1 = c1.material.restfullness;
    let re2 = c2.material.restfullness;

    let r1 = c1.get_radius(t1.scale);
    let r2 = c2.get_radius(t2.scale);

    // pop
    let diff = w2 - w1;
    let distance_pop = diff.magnitude() - r1 - r2;

    // just in case that w1 == w2
    let normal = if diff == Vec3::zero() {
        Vec3::unit_y()
    } else {
        diff.normalized()
    };
    debug_assert_finite!(normal);

    pop_coliders(distance_pop * normal, t1, t2, &rb1, &rb2);
    standard_collision(normal, rb1, rb2, -normal * r1, normal * r2, re1, re2);
}

pub fn collide_sphere_vs_box(
    c1: &SphereColider,
    rb1: &mut RidgidBody,
    t1: &mut Transform,
    w1: Vec3, // world position
    c2: &BoxColider,
    rb2: &mut RidgidBody,
    t2: &mut Transform,
    w2: Vec3, // world position
) {
    let re1 = c1.material.restfullness;
    let re2 = c2.material.restfullness;

    let r = c1.get_radius(t1.scale);
    debug_assert!(r > 0.0);

    let scale = t2.scale * c2.scale;
    debug_assert_finite!(scale);

    let closest_point = get_closest_point(w1, w2, scale, t2.rotation);
    debug_assert_finite!(closest_point);

    let overlap_distance = r - closest_point.distance(w1);
    debug_assert!(overlap_distance >= 0.0);

    // if objects completely overlap
    let normal = if r <= overlap_distance {
        if w1 == w2 {
            Vec3::unit_y()
        } else {
            (w2 - w1).normalized()
        }
    } else {
        (w1 - closest_point).normalized()
    };

    debug_assert_finite!(normal);

    let point_of_contact = closest_point;

    pop_coliders(normal * overlap_distance, t1, t2, &rb1, &rb2);

    standard_collision(
        normal,
        rb1,
        rb2,
        point_of_contact - w1,
        point_of_contact - w2,
        re1,
        re2,
    );
}