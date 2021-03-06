use common::{Mat3, Transform, Vec3};

use crate::{
    cube::collision::collide_cube_vs_cube,
    cube::{collision::is_colliding_cube_vs_cube, CubeCollider},
    get_position,
    macros::debug_assert_finite,
    macros::debug_assert_normalized,
    sphere::collision::collide_sphere_vs_sphere,
    sphere::collision::{is_colliding_sphere_vs_cube, is_colliding_sphere_vs_sphere},
    sphere::{collision::collide_sphere_vs_cube, SphereCollider},
    PhysicsMaterial, Rigidbody,
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Collider {
    Sphere(SphereCollider),
    Cube(CubeCollider),
}

impl Collider {
    pub fn inv_inertia_tensor(&self) -> Mat3 {
        match self {
            Self::Sphere(a) => a.inv_inertia_tensor(),
            Self::Cube(a) => a.inv_inertia_tensor(),
        }
    }
}

/// Returns true if 2 objects are colliding
pub fn is_colliding(c1: &Collider, t1: &Transform, c2: &Collider, t2: &Transform) -> bool {
    let w1 = get_position(t1, c1);
    let w2 = get_position(t2, c2);

    debug_assert_finite!(w1);
    debug_assert_finite!(w2);

    match (c1, c2) {
        (Collider::Sphere(sc1), Collider::Sphere(sc2)) => {
            is_colliding_sphere_vs_sphere(w1, w2, sc1, t1, sc2, t2)
        }
        (Collider::Cube(bc1), Collider::Cube(bc2)) => {
            is_colliding_cube_vs_cube(w1, w2, bc1, t1, bc2, t2)
        }
        (Collider::Sphere(sc), Collider::Cube(bc)) => {
            is_colliding_sphere_vs_cube(w1, w2, sc, t1, bc, t2)
        }
        (Collider::Cube(bc), Collider::Sphere(sc)) => {
            is_colliding_sphere_vs_cube(w2, w1, sc, t2, bc, t1)
        }
    }
}

pub fn bounce(input: Vec3, normal: Vec3) -> Vec3 {
    fn proj(on: Vec3, vec: Vec3) -> Vec3 {
        vec.dot(on) * on / on.magnitude_squared()
    }

    input - 2.0 * proj(normal, input)
}

pub fn standard_collision(
    normal: Vec3,
    rb: (&mut Rigidbody, &mut Rigidbody),
    // coll: (&Collider, &Collider),
    trans: (&Transform, &Transform),
    // inverted inertia matrices
    inertia: (Mat3, Mat3),
    // offset from point of contact
    r: (Vec3, Vec3),
    mat: (&PhysicsMaterial, &PhysicsMaterial),
) {
    debug_assert_finite!(normal);
    debug_assert_normalized!(normal);

    // see this link for explanation of all the math, variables are all named according to this article
    // lowercase omega is substituted with w in this code.
    // https://en.wikipedia.org/wiki/Collision_response#Impulse-Based_Reaction_Model

    // all these calculations are done the same way for the two objects, so it's separated out for clarity
    // v_i, m_i, w_i, v_pi, inertia, inertia term
    let do_calcs = |rb: &mut Rigidbody,
                    inertia: Mat3,
                    rot: Mat3,
                    r: Vec3,
                    n: Vec3|
     -> (f32, Vec3, Mat3, Vec3) {
        let v = rb.velocity();
        let m = rb.mass;
        // inertia tensor in world space coordinates
        let i = rot * inertia * rot.transposed();
        let w = rb.angular_velocity(i);
        let v_p = v + w.cross(r);
        let i_term = (i * r.cross(n)).cross(r);

        (m, v_p, i, i_term)
    };

    let (m_1, v_p1, i_1, i_term_1) =
        do_calcs(rb.0, inertia.0, Mat3::from(trans.0.rotation), r.0, normal);
    let (m_2, v_p2, i_2, i_term_2) =
        do_calcs(rb.1, inertia.1, Mat3::from(trans.1.rotation), r.1, normal);

    let v_r = v_p1 - v_p2;

    // the divisor in the j_r calculation (factored out for readability)
    let divisor = if rb.0.is_static {
        (1.0 / m_2) + i_term_2.dot(normal)
    } else if rb.1.is_static {
        (1.0 / m_1) + i_term_1.dot(normal)
    } else {
        (1.0 / m_1) + (1.0 / m_2) + (i_term_1 + i_term_2).dot(normal)
    };

    // TODO make make this correct, idk if (c1+c2)/2 is correct
    let e = (mat.0.restfullness + mat.1.restfullness) / 2.0; // bounce factor 1.0 = bounce 0 = no bounce
    let u = (mat.0.friction + mat.1.friction) / 2.0; // friction

    // impulse magnitude
    let j_r = -(1.0 + e) * v_r.dot(normal) / divisor;

    let epsilon = 0.001;
    // rb, tangent, inertia tensor, offset, forces
    let do_friction = |rb: &mut Rigidbody, i: Mat3, r: Vec3, _t: &Transform| {
        let relative_velocity = rb.velocity() + rb.angular_velocity(i).cross(r);

        let tangent_velocity = relative_velocity - normal * relative_velocity.dot(normal);

        if tangent_velocity.magnitude_squared() < epsilon * epsilon {
            return;
        }

        let t = tangent_velocity.normalized();

        let vt = relative_velocity.dot(t);
        let kt = rb.mass.recip() + r.cross(t).dot(i * r.cross(t));

        let b = (u * j_r).abs();

        let jt = (-vt / kt).clamp(-b, b);

        rb.linear_momentum += jt * t;
        rb.angular_momentum += jt * r.cross(t);
    };

    if !rb.0.is_static {
        rb.0.linear_momentum += j_r * normal / m_1;
        rb.0.angular_momentum += -j_r * (i_1 * r.0.cross(normal));
        do_friction(rb.0, i_1, r.0, trans.0);
    }
    if !rb.1.is_static {
        rb.1.linear_momentum += -j_r * normal / m_2;
        rb.1.angular_momentum += -j_r * (i_2 * r.1.cross(normal));
        do_friction(rb.1, i_2, r.1, trans.1);
    }
}

/// where normal_distance is the normal pointing at c1 from c2 with the length of the intercetion
pub fn pop_colliders(
    normal_distance: Vec3,
    t1: &mut Transform,
    t2: &mut Transform,
    rb1: &Rigidbody,
    rb2: &Rigidbody,
) {
    //debug_assert!(normal_distance.magnitude_squared() > 0.0); //TODO
    // cant move static colliders
    if rb1.is_static && rb2.is_static {
        return;
    }

    const POP_SIZE: f32 = 1.1;
    let pop = normal_distance * POP_SIZE;
    if rb1.is_static {
        t2.position -= pop;
    } else if rb2.is_static {
        t1.position += pop;
    } else {
        t2.position -= pop * 0.5;
        t1.position += pop * 0.5;
    }
}

pub fn solve_colliding(
    c1: &Collider,
    rb1: &mut Rigidbody,
    t1: &mut Transform,
    c2: &Collider,
    rb2: &mut Rigidbody,
    t2: &mut Transform,
) {
    let w1 = get_position(t1, c1);
    let w2 = get_position(t2, c2);

    debug_assert_finite!(w1);
    debug_assert_finite!(w2);

    match (c1, c2) {
        (Collider::Sphere(sc1), Collider::Sphere(sc2)) => {
            collide_sphere_vs_sphere(sc1, rb1, t1, w1, sc2, rb2, t2, w2)
        }
        (Collider::Cube(bc1), Collider::Cube(bc2)) => {
            collide_cube_vs_cube(bc1, rb1, t1, w1, bc2, rb2, t2, w2)
        }
        (Collider::Sphere(sc), Collider::Cube(bc)) => {
            collide_sphere_vs_cube(sc, rb1, t1, w1, bc, rb2, t2, w2)
        }
        (Collider::Cube(bc), Collider::Sphere(sc)) => {
            collide_sphere_vs_cube(sc, rb2, t2, w2, bc, rb1, t1, w1)
        }
    }
}

// Returns `true` if there was a collision
pub fn collide(
    t1: &mut Transform,
    rb1: &mut Rigidbody,
    c1: &Collider,
    t2: &mut Transform,
    rb2: &mut Rigidbody,
    c2: &Collider,
) {
    if rb1.is_static && rb2.is_static {
        return;
    }

    if is_colliding(c1, t1, c2, t2) {
        if cfg!(debug_assertions) {
            let rot_1 = Mat3::from(t1.rotation);
            let rot_2 = Mat3::from(t2.rotation);

            let i_1 = rot_1 * c1.inv_inertia_tensor() * rot_1.transposed();
            let i_2 = rot_2 * c2.inv_inertia_tensor() * rot_2.transposed();

            debug_assert_finite!(rb1.velocity());
            debug_assert_finite!(rb2.velocity());
            debug_assert_finite!(rb1.angular_velocity(i_1));
            debug_assert_finite!(rb2.angular_velocity(i_2));
        }

        solve_colliding(c1, rb1, t1, c2, rb2, t2);
    }
}
