use common::{Ray, Transform, Vec3};

use crate::{
    get_position,
    macros::{debug_assert_finite, debug_assert_normalized},
    r#box::mesh::{get_normal_from_tri, get_tris_for_box, get_verts},
    BoxCollider, Collider, SphereCollider,
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct RayCastHit {
    pub distance: f32,
    /// normalized
    pub normal: Vec3,
}

impl RayCastHit {
    pub fn new(distance: f32, normal: Vec3) -> Self {
        Self { distance, normal }
    }
}

pub fn raycast(t: &Transform, cols: &Vec<Collider>, ray: Ray) -> Option<RayCastHit> {
    debug_assert_normalized!(ray.direction);
    debug_assert_finite!(ray.origin);

    let mut distance = f32::INFINITY;
    let mut normal = Vec3::zero();

    for c in cols {
        let w = get_position(t, c);
        if let Some(hit) = raycast_collider(t, c, Ray::new(ray.origin - w, ray.direction)) {
            if hit.distance < distance {
                distance = hit.distance;
                normal = hit.normal;
                debug_assert_normalized!(hit.normal);
            }
        }
    }

    if distance < f32::INFINITY {
        Some(RayCastHit::new(distance, normal))
    } else {
        None
    }
}

/// rotation, collider, ray -> distance, normal
pub fn raycast_collider(t: &Transform, c: &Collider, ray: Ray) -> Option<RayCastHit> {
    match c {
        Collider::Sphere(s) => raycast_sphere(t, s, ray),
        Collider::Box(b) => raycast_box(t, b, ray),
    }
}

pub fn raycast_box(t: &Transform, c: &BoxCollider, ray: Ray) -> Option<RayCastHit> {
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

pub fn raycast_sphere(t1: &Transform, c: &SphereCollider, ray: Ray) -> Option<RayCastHit> {
    let origen = ray.origin;

    let r = c.get_radius(t1.scale);
    let t = (-origen).dot(ray.direction);
    let p = origen + ray.direction * t;

    let y = p.magnitude();

    if y < r {
        let x = (r * r - y * y).sqrt();
        let poc_t = t - x;
        let poc = origen + ray.direction * poc_t;
        Some(RayCastHit::new(poc_t, poc.normalized()))
    } else {
        None
    }
}
