use crate::physics::macros::debug_assert_normalized;
use crate::rendering::Line;
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static! {
    pub static ref LINE_ATLAS: Mutex<HashMap<String, Line>> = Mutex::new(HashMap::new());
}

use super::{
    get_position,
    r#box::collision::is_colliding_box_vs_box,
    sphere::collision::{is_colliding_sphere_vs_box, is_colliding_sphere_vs_sphere},
    Collider, PhysicsMaterial, PhysicsObject, RidgidBody,
};
use crate::physics::sphere::collision::collide_sphere_vs_sphere;
use crate::physics::{
    macros::debug_assert_finite, proj, r#box::collision::collide_box_vs_box,
    sphere::collision::collide_sphere_vs_box,
};

use common::{Mat3, Ray, Transform, Vec3};

/// Returns true if 2 objects are colliding
#[must_use]
pub fn is_colliding(c1: &Collider, t1: &Transform, c2: &Collider, t2: &Transform) -> bool {
    let w1 = get_position(t1, c1);
    let w2 = get_position(t2, c2);

    debug_assert_finite!(w1);
    debug_assert_finite!(w2);

    match c1 {
        Collider::SphereColider(sc1) => match c2 {
            Collider::SphereColider(sc2) => is_colliding_sphere_vs_sphere(w1, w2, sc1, t1, sc2, t2),
            Collider::BoxColider(bc2) => is_colliding_sphere_vs_box(w1, w2, sc1, t1, bc2, t2),
        },
        Collider::BoxColider(bc1) => match c2 {
            Collider::BoxColider(bc2) => is_colliding_box_vs_box(w1, w2, bc1, t1, bc2, t2),
            Collider::SphereColider(_) => is_colliding(c2, t2, c1, t1), // reuse code
        },
    }
}

pub fn bounce(input: Vec3, normal: Vec3) -> Vec3 {
    return input - 2.0 * proj(normal, input);
}

pub fn standard_collision(
    normal: Vec3,
    rb: (&mut RidgidBody, &mut RidgidBody),
    //coll: (&Collider, &Collider),
    trans: (&Transform, &Transform),
    // inverted inertia matrices
    inertia: (Mat3, Mat3),
    // offset from point of contact
    r: (Vec3, Vec3),
    mat: (&PhysicsMaterial, &PhysicsMaterial),
) {
    //println!("POP");

    debug_assert_finite!(normal);
    debug_assert_normalized!(normal);

    // see this link for explanation of all the math, variables are all named according to this article
    // lowercase omega is substituted with w in this code.
    // https://en.wikipedia.org/wiki/Collision_response#Impulse-Based_Reaction_Model

    /*match coll {
        (Collider::SphereColider(_), Collider::SphereColider(_)) => {}
        (Collider::SphereColider(_), Collider::BoxColider(_)) => {}
        (Collider::BoxColider(_), Collider::SphereColider(_)) => {}
        (Collider::BoxColider(_), Collider::BoxColider(_)) => {}
    }*/

    let cross = |a, b| Vec3::cross(a, b);
    let dot = |a, b| Vec3::dot(a, b);

    // all these calculations are done the same way for the two objects, so it's separated out for clarity
    // v_i, m_i, w_i, v_pi, inertia, inertia term
    let do_calcs = |rb: &mut RidgidBody,
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
        let v_p = v + cross(w, r);
        let i_term = cross(i * cross(r, n), r);

        (m, v_p, i, i_term)
    };

    let (m_1, v_p1, i_1, i_term_1) =
        do_calcs(rb.0, inertia.0, Mat3::from(trans.0.rotation), r.0, normal);
    let (m_2, v_p2, i_2, i_term_2) =
        do_calcs(rb.1, inertia.1, Mat3::from(trans.1.rotation), r.1, normal);

    let v_r = v_p1 - v_p2;

    // the divisor in the j_r calculation (factored out for readability)
    let divisor = if rb.0.is_static {
        (1.0 / m_2) + dot(i_term_2, normal)
    } else if rb.1.is_static {
        (1.0 / m_1) + dot(i_term_1, normal)
    } else {
        (1.0 / m_1) + (1.0 / m_2) + dot(i_term_1 + i_term_2, normal)
    };

    // TODO make make this correct, idk if (c1+c2)/2 is correct
    let e = (mat.0.restfullness + mat.1.restfullness) / 2.0; // bounce factor 1.0 = bounce 0 = no bounce
    let u = (mat.0.friction + mat.1.friction) / 2.0; // friction

    // impulse magnitude
    let j_r = dot(-(1.0 + e) * v_r, normal) / divisor;

    // relative velocity, normal, sum of all external forces
    fn compute_tangent(v_r: Vec3, n: Vec3, f_e: Vec3) -> Vec3 {
        if !(v_r.dot(n).abs() < f32::EPSILON) {
            let a = v_r - v_r.dot(n) * n;
            a / a.magnitude()
        } else if !(f_e.dot(n).abs() < f32::EPSILON) {
            let a = f_e - f_e.dot(n) * n;
            a / a.magnitude()
        } else {
            Vec3::zero()
        }
    }

    let f_e1 = rb.0.acceleration * m_1;
    let f_e2 = rb.1.acceleration * m_2;

    let v_r1 = rb.0.velocity() + rb.0.angular_velocity(i_1).cross(r.0);
    let v_r2 = rb.1.velocity() + rb.1.angular_velocity(i_2).cross(r.1);

    let t1 = compute_tangent(v_r1, normal, f_e1);
    let t2 = compute_tangent(v_r2, normal, f_e2);

    let epsilon = 0.001;
    // rb, tangent, inertia tensor, offset, forces
    let do_friction = |rb: &mut RidgidBody, t: Vec3, i: Mat3, r: Vec3, _t: &Transform| {
        let relative_velocity = rb.velocity() + rb.angular_velocity(i).cross(r);

        let tangent_velocity = relative_velocity - normal * relative_velocity.dot(normal);

        if tangent_velocity.magnitude_squared() < epsilon * epsilon {
            return;
        }

        let diff = tangent_velocity.normalized() - t;
        if diff.magnitude() > 0.01 {
            //println!("tangent_vel: {}, t: {}", tangent_velocity.normalized(), t);
        }

        let t = tangent_velocity.normalized();

        let vt = relative_velocity.dot(t);
        let kt = rb.mass.recip() + r.cross(t).dot(i * r.cross(t));

        let b = (u * j_r).abs();

        let jt = (-vt / kt).clamp(-b, b);
        let add_linj = j_r * normal;
        set_line(
            rb.id,
            "JR",
            Line {
                start: _t.position,
                end: _t.position + add_linj,
                color: Vec3::new(0.0, 1.0, 0.0),
            },
        );

        rb.linear_momentum += jt * t;
        rb.angular_momentum += jt * r.cross(t);
    };

    if !rb.0.is_static {
        rb.0.linear_momentum += j_r * normal / m_1;
        rb.0.angular_momentum += -j_r * (i_1 * cross(r.0, normal));
        do_friction(rb.0, t1, i_1, r.0, trans.0);
    }
    if !rb.1.is_static {
        rb.1.linear_momentum += -j_r * normal / m_2;
        rb.1.angular_momentum += -j_r * (i_2 * cross(r.1, normal));

        do_friction(rb.1, t2, i_2, r.1, trans.1);
    }
}

pub fn set_line(id: usize, key: &str, line: Line) {
    set_line_key(format!("{} {}", id, key), line);
}

pub fn clear_lines() {
    LINE_ATLAS.lock().unwrap().clear();
}

pub fn set_line_key(key: String, line: Line) {
    LINE_ATLAS.lock().unwrap().insert(key, line);
}

/// where normal_distance is the normal pointing at c1 from c2 with the length of the intercetion
pub fn pop_coliders(
    normal_distance: Vec3,
    t1: &mut Transform,
    t2: &mut Transform,
    rb1: &RidgidBody,
    rb2: &RidgidBody,
) {
    //debug_assert!(normal_distance.magnitude_squared() > 0.0); //TODO
    // cant move static coliders
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
    rb1: &mut RidgidBody,
    t1: &mut Transform,
    c2: &Collider,
    rb2: &mut RidgidBody,
    t2: &mut Transform,
    dt: f32,
) {
    let w1 = get_position(t1, c1);
    let w2 = get_position(t2, c2);

    debug_assert_finite!(w1);
    debug_assert_finite!(w2);

    match c1 {
        Collider::SphereColider(b1) => match c2 {
            Collider::SphereColider(b2) => {
                collide_sphere_vs_sphere(b1, rb1, t1, w1, b2, rb2, t2, w2)
            }
            Collider::BoxColider(b2) => collide_sphere_vs_box(b1, rb1, t1, w1, b2, rb2, t2, w2, dt),
        },
        Collider::BoxColider(b1) => match c2 {
            Collider::SphereColider(b2) => {
                collide_sphere_vs_box(b2, rb2, t2, w2, b1, rb1, t1, w1, dt)
            }
            Collider::BoxColider(b2) => collide_box_vs_box(b1, rb1, t1, w1, b2, rb2, t2, w2),
        },
    }
}

pub fn collide(
    rb1: &mut RidgidBody,
    t1: &mut Transform,
    cc1: &Vec<Collider>,
    t2: &mut Transform,
    rb2: &mut RidgidBody,
    cc2: &Vec<Collider>,
    dt: f32,
) -> bool {
    let mut has_colided = false;

    for c1 in cc1 {
        for c2 in cc2 {
            if is_colliding(c1, t1, c2, t2) {
                has_colided = true;

                let rot_1 = Mat3::from(t1.rotation);
                let rot_2 = Mat3::from(t2.rotation);

                let i_1 = rot_1 * c1.inv_inertia_tensor() * rot_1.transposed();
                let i_2 = rot_2 * c2.inv_inertia_tensor() * rot_2.transposed();

                debug_assert_finite!(rb1.velocity());
                debug_assert_finite!(rb2.velocity());
                debug_assert_finite!(rb1.angular_velocity(i_1));
                debug_assert_finite!(rb2.angular_velocity(i_2));

                rb1.is_active = true;
                rb2.is_active = true;
                rb1.is_active_time = 0.0;
                rb2.is_active_time = 0.0;

                solve_colliding(c1, rb1, t1, c2, rb2, t2, dt);
            }
        }
    }

    rb1.is_colliding_this_frame = has_colided || rb1.is_colliding_this_frame;
    rb2.is_colliding_this_frame = has_colided || rb2.is_colliding_this_frame;

    has_colided
}

impl RidgidBody {
    pub fn add_impulse(&mut self, force: Vec3) {
        self.linear_momentum += force;
    }

    pub fn add_acceleration(&mut self, acceleration: Vec3, dt: f32) {
        self.linear_momentum += acceleration * self.mass * dt;
    }

    pub fn add_impulse_at_location(&mut self, pos: Vec3, impulse: Vec3, location: Vec3) {
        debug_assert_finite!(impulse);
        debug_assert_finite!(location);

        // TODO: fix this function to properly add angular/linear momentum

        //debug_assert!(velocity.magnitude_squared() != 0.0, "velocity is too close to 0 = {}", velocity);

        self.linear_momentum += impulse;
        let r = location - pos;
        self.angular_momentum += r.cross(impulse);

        //https://en.wikipedia.org/wiki/Angular_velocity
    }

    pub fn apply_forces(&self) -> Vec3 {
        let grav_acc = Vec3::new(0.0, -9.81, 0.0); // 9.81m/s^2 down in the Y-axis // TODO MAKE CONSTANT
        debug_assert!(self.drag.is_finite());
        debug_assert_finite!(self.velocity());
        debug_assert!(
            self.velocity().magnitude().is_finite(),
            "magnitude is infinite, velocity = {}",
            self.velocity()
        );
        let drag_force = 0.5 * self.drag * (self.velocity() * self.velocity().magnitude()); // D = 0.5 * (rho * C * Area * vel^2)
        debug_assert!(self.mass.is_finite() && self.mass != 0.0);
        debug_assert_finite!(drag_force);
        let drag_acc = drag_force / self.mass; // a = F/m
        debug_assert_finite!(drag_acc);
        return grav_acc - drag_acc;
    }

    pub fn step(&mut self, dt: f32, transform: &mut Transform, inv_inertia_tensor: Mat3) {
        if self.is_static {
            return;
        }

        debug_assert_finite!(self.velocity());

        //https://en.wikipedia.org/wiki/Verlet_integration
        transform.position += self.velocity() * dt; // * dt + self.acceleration * (dt * dt * 0.5);
        debug_assert_finite!(transform.position);
        //self.acceleration = self.apply_forces();
        //debug_assert_finite!(self.acceleration);

        self.add_impulse(self.acceleration * dt * self.mass);
        //self.add_acceleration(self.acceleration, dt);

        // apply rotation
        let i_inv = Mat3::from(transform.rotation)
            * inv_inertia_tensor
            * Mat3::from(transform.rotation).transposed();
        let angular_velocity = self.angular_velocity(i_inv);

        transform.rotation.rotate_x(angular_velocity.x * dt);
        transform.rotation.rotate_y(angular_velocity.y * dt);
        transform.rotation.rotate_z(angular_velocity.z * dt);

        self.is_active_time += dt;
    }
}

pub fn update(
    is_paused: &mut bool,
    dt: f32,
    transforms: &mut Vec<Transform>,
    phx_objects: &mut Vec<PhysicsObject>,
) {
    //println!("===");
    let real_dt = dt;
    let phx_length = phx_objects.len();

    for i in 0..phx_length {
        phx_objects[i].rb.is_colliding_this_frame = false;

        // update last frame location
        phx_objects[i].rb.last_frame_location = transforms[i].position;
        phx_objects[i].rb.last_frame_rotation = transforms[i].rotation;

        let tensor = phx_objects[i].colliders[0].inv_inertia_tensor();

        // simulate one step in the simulation
        phx_objects[i].rb.step(real_dt, &mut transforms[i], tensor);
    }
    for i in 0..phx_length {
        let (phx_first, phx_last) = phx_objects.split_at_mut(i + 1);
        //if phx_first[i].rb.is_static || !phx_first[i].rb.is_active {
        //    continue; // we dont care about non active or static objects
        //}

        let (trans_first, trans_last) = transforms.split_at_mut(i + 1);

        let mut has_colided = false;

        // pop coliders and apply force on all colliding objects
        for (transform, phx_obj) in trans_last.iter_mut().zip(phx_last.iter_mut()) {
            if collide(
                &mut phx_first[i].rb,
                &mut trans_first[i],
                &phx_first[i].colliders,
                transform,
                &mut phx_obj.rb,
                &phx_obj.colliders,
                dt,
            ) {
                //if i == 3 {
                // println!("col >> {} = {} | {}",i, phx_first[i].rb.is_colliding_this_frame, phx_obj.rb.is_colliding_this_frame);
                // }
                has_colided = true;
            }
        }
        // if i == 3 {
        // println!("col {}",phx_first[i].rb.is_colliding_this_frame);
        //}
        if has_colided {
            *is_paused = true;
        }
    }
    for i in 0..phx_length {
        let direction = Vec3::new(-1.0, -1.0, -1.0).normalized();
        let origin = Vec3::new(10.0, 13.0, 10.0);
        if let Some(hit) = super::raycast(
            &transforms[i],
            &phx_objects[i].colliders,
            Ray::new(origin, direction),
        ) {
            set_line(
                i,
                "ray",
                Line {
                    start: origin,
                    end: origin + direction * hit.distance,
                    color: Vec3::new(0.0, 1.0, 0.0),
                },
            );
            set_line(
                i,
                "rayNormal",
                Line {
                    start: origin + direction * hit.distance,
                    end: origin + direction * hit.distance + hit.normal,
                    color: Vec3::new(1.0, 0.0, 0.0),
                },
            );
        } else {
            set_line(
                i,
                "ray",
                Line {
                    start: origin,
                    end: direction * 1000.0,
                    color: Vec3::new(0.0, 0.0, 1.0),
                },
            );
        }

        // if phx_objects[i].rb.is_colliding_this_frame {
        //     println!("col >> {} = {}",i, phx_objects[i].rb.is_colliding_this_frame);
        //}
        phx_objects[i].rb.is_colliding = phx_objects[i].rb.is_colliding_this_frame;

        // if i == 1 {
        //      println!("{}", phx_objects[i].rb.velocity());
        // }
        set_line(
            i,
            "vel",
            Line {
                start: transforms[i].position,
                end: transforms[i].position + phx_objects[i].rb.velocity(),
                color: Vec3::new(1.0, 0.0, 0.0),
            },
        );
    }
}
