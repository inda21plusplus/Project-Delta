use crate::rendering::Line;
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static! {
    pub static ref LINE_ATLAS: Mutex<HashMap<String, Line>> = Mutex::new(HashMap::new());
}

use super::{
    get_position,
    r#box::{collision::is_colliding_box_vs_box, BoxColider},
    sphere::collision::{is_colliding_sphere_vs_box, is_colliding_sphere_vs_sphere},
    Collider, PhysicsObject, RidgidBody,
};
use crate::physics::sphere::collision::collide_sphere_vs_sphere;
use crate::physics::{
    is_finite, macros::debug_assert_finite, proj, r#box::collision::collide_box_vs_box,
    sphere::collision::collide_sphere_vs_box,
};

use common::{Mat3, Transform, Vec3};

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

#[inline]
#[must_use]
/// using https://en.wikipedia.org/wiki/Elastic_collision on a 1d plane where m is mass and v is velocity
fn standard_elastic_collision(m1: f32, v1: f32, m2: f32, v2: f32) -> (f32, f32) {
    let u1: f32 = (m1 * v1 - m2 * v1 + 2.0 * m2 * v2) / (m1 + m2);
    let u2: f32 = (2.0 * m1 * v1 - m1 * v2 + m2 * v2) / (m1 + m2);

    //todo https://en.wikipedia.org/wiki/Inelastic_collision
    //todo https://en.wikipedia.org/wiki/Coefficient_of_restitution
    (u1, u2)
}

#[must_use]
fn standard_elastic_collision_3(m1: f32, v1: &Vec3, m2: f32, v2: &Vec3) -> (Vec3, Vec3) {
    let (v1x, v2x) = standard_elastic_collision(m1, v1.x, m2, v2.x);
    let (v1y, v2y) = standard_elastic_collision(m1, v1.y, m2, v2.y);
    let (v1z, v2z) = standard_elastic_collision(m1, v1.z, m2, v2.z);
    (Vec3::new(v1x, v1y, v1z), Vec3::new(v2x, v2y, v2z))
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
    // not used atm, restfullness
    _re1: f32,
    _re2: f32,
) {
    debug_assert_finite!(normal);
    // see this link for explanation of all the math, variables are all named according to this article
    // lowercase omega is substituted with w in this code.
    // https://en.wikipedia.org/wiki/Collision_response#Impulse-Based_Reaction_Model

    /*match coll {
        (Collider::SphereColider(_), Collider::SphereColider(_)) => {}
        (Collider::SphereColider(_), Collider::BoxColider(_)) => {}
        (Collider::BoxColider(_), Collider::SphereColider(_)) => {}
        (Collider::BoxColider(_), Collider::BoxColider(_)) => {}
    }*/

    let normal = -normal;

    let cross = |a, b| Vec3::cross(a, b);
    let dot = |a, b| Vec3::dot(a, b);

    // all these calculations are done the same way for the two objects, so it's separated out for clarity
    // v_i, m_i, w_i, v_pi, inertia, inertia term
    let do_calcs =
        |rb: &mut RidgidBody, inertia: Mat3, rot: Mat3, r: Vec3| -> (f32, Vec3, Mat3, Vec3) {
            let v = rb.velocity();
            let m = rb.mass;
            // inertia tensor in world space coordinates
            let i = rot * inertia * rot.transposed();
            let w = rb.angular_velocity(i);
            let v_p = v + cross(w, r);
            let i_term = cross(i * cross(r, normal), r);

            (m, v_p, i, i_term)
        };

    let (m_1, v_p1, i_1, i_term_1) = do_calcs(rb.0, inertia.0, Mat3::from(trans.0.rotation), r.0);
    let (m_2, v_p2, i_2, i_term_2) = do_calcs(rb.1, inertia.1, Mat3::from(trans.1.rotation), r.1);

    let v_r = v_p1 - v_p2;

    // the divisor in the j_r calculation (factored out for readability)
    let divisor = (1.0 / m_1) + (1.0 / m_2) + dot(i_term_1 + i_term_2, normal);

    let e = 0.01;

    // impulse magnitude
    let j_r = (dot(-(1.0 + e) * v_r, normal) / divisor);

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
    let v_r2 = rb.1.velocity() + rb.1.angular_velocity(i_1).cross(r.1);

    let t1 = compute_tangent(v_r1, normal, f_e1);
    let t2 = compute_tangent(v_r2, normal, f_e2);

    let u = 0.5;

    // rb, tangent, inertia tensor, offset, forces
    let do_friction = |rb: &mut RidgidBody, t: Vec3, i: Mat3, r: Vec3, f_e: Vec3| {
        let relative_velocity = rb.velocity() + rb.angular_velocity(i).cross(r);

        let tangent_velocity = relative_velocity - normal * relative_velocity.dot(normal);

        if tangent_velocity.magnitude_squared() < f32::EPSILON * f32::EPSILON {
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

        rb.linear_momentum += jt * t;
        rb.angular_momentum += jt * r.cross(t);
    };

    if !rb.0.is_static {
        rb.0.linear_momentum += j_r;
        rb.0.angular_momentum += j_r * cross(r.0, normal);

        do_friction(rb.0, t1, i_1, v_r, f_e1);

        let vel = rb.0.velocity();

        set_line(
            rb.0.id,
            "vel",
            Line {
                start: trans.0.position,
                end: trans.0.position + vel,
                color: Vec3::new(1.0, 0.0, 0.0),
            },
        );
    }
    if !rb.1.is_static {
        rb.1.linear_momentum += -j_r * normal;
        rb.1.angular_momentum += j_r * cross(r.1, normal);

        do_friction(rb.1, t2, i_2, v_r, f_e2);

        let vel = rb.1.velocity();

        set_line(
            rb.1.id,
            "vel",
            Line {
                start: trans.1.position,
                end: trans.1.position + vel,
                color: Vec3::new(1.0, 0.0, 0.0),
            },
        );
    }
}

pub fn set_line(id: usize, key: &str, line: Line) {
    LINE_ATLAS
        .lock()
        .unwrap()
        .insert(format!("{} {}", id, key), line);
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
    let t1_post = t1.position;
    let t2_post = t2.position;

    let mut has_colided = false;

    // used when pop does not work, simply uses binary search to pop the colliders using lastframe location and current location
    let mut post_fix: Vec<Vec<usize>> = Vec::with_capacity(cc1.len());

    for c1_index in 0..cc1.len() {
        let c1 = &cc1[c1_index];

        let mut current_post_fix: Vec<usize> = Vec::new();
        for c2_index in 0..cc2.len() {
            let c2 = &cc2[c2_index];
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
                //solve_colliding(c1, rb1, t1, c2, rb2, t2);
                match (c1, c2) {
                    // (Collider::BoxColider(_), Collider::BoxColider(_)) => {
                    //     current_post_fix.push(c2_index);
                    // }
                    _ => {
                        solve_colliding(c1, rb1, t1, c2, rb2, t2, dt);
                    }
                }
            }
        }
        post_fix.push(current_post_fix);
    }

    // binary searches the col point

    const BINARY_ITTERATIONS: i32 = 8;
    // TODO ADD ROTATION

    let t1_pre = rb1.last_frame_location;
    let t2_pre = rb2.last_frame_location;

    for c1_index in 0..cc1.len() {
        if post_fix[c1_index].is_empty() {
            continue;
        }

        let c1 = &cc1[c1_index];
        let mut search_location = 0.5f32;
        let mut search_length = 0.5f32;

        let mut is_not_touching = 0.0f32;
        let mut is_touching = 1.0f32;

        for _ in 0..BINARY_ITTERATIONS {
            t1.position = t1_post * search_location + t1_pre * (1.0 - search_location);
            t2.position = t2_post * search_location + t2_pre * (1.0 - search_location);

            search_length *= 0.5;

            let mut colliding = false;
            for c2_index in &post_fix[c1_index] {
                let c2 = &cc2[*c2_index];
                if is_colliding(c1, t1, c2, t2) {
                    colliding = true;
                    break;
                }
            }
            if !colliding {
                is_not_touching = search_location;
            } else {
                is_touching = search_location;
            }
            search_location += search_length * if colliding { -1.0 } else { 1.0 };
        }

        t1.position = t1_post * is_touching + t1_pre * (1.0 - is_touching);
        t2.position = t2_post * is_touching + t2_pre * (1.0 - is_touching);

        for c2_index in &post_fix[c1_index] {
            let c2 = &cc2[*c2_index];

            solve_colliding(c1, rb1, t1, c2, rb2, t2, dt);
            has_colided = true;
        }
        t1.position = t1_post * is_not_touching + t1_pre * (1.0 - is_not_touching);
        t2.position = t2_post * is_not_touching + t2_pre * (1.0 - is_not_touching);
    }

    rb1.is_colliding_this_frame = has_colided || rb1.is_colliding_this_frame;
    rb2.is_colliding_this_frame = has_colided || rb2.is_colliding_this_frame;

    has_colided
}

impl RidgidBody {
    pub fn add_impulse(&mut self, force: Vec3) {
        self.linear_momentum += force;
    }

    pub fn add_impulse_at_location(&mut self, pos: Vec3, impulse: Vec3, location: Vec3) {
        debug_assert_finite!(impulse);
        debug_assert_finite!(location);

        // TODO: fix this function to properly add angular/linear momentum

        //debug_assert!(velocity.magnitude_squared() != 0.0, "velocity is too close to 0 = {}", velocity);

        // TODO: can we please remove this
        // if zero velocity is applied then nothing happends
        if impulse.magnitude_squared() == 0.0 {
            return;
        }

        self.linear_momentum += impulse;
        let r = location - pos;
        self.angular_momentum += r.cross(impulse);

        //https://en.wikipedia.org/wiki/Angular_velocity

        // just random shit
        return;
        let offset = self.center_of_mass_offset + location;
        let normal = offset;

        let rotation_around = -(normal.normalized().cross(impulse.normalized())).normalized();
        debug_assert!(
            is_finite(&rotation_around),
            "rotation_around = {} normal {} velocity {}",
            rotation_around,
            normal,
            impulse
        );

        let torque = rotation_around * 10.0; //velocity *  / offset.magnitude();

        //self.angular_velocity += torque;

        // TODO idk what angular_velocity is
    }

    pub fn apply_forces(&self) -> Vec3 {
        let grav_acc = Vec3::new(0.0, -9.81, 0.0); // 9.81m/s^2 down in the Z-axis // TODO MAKE CONSTANT
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
        transform.position += self.velocity() * dt + self.acceleration * (dt * dt * 0.5);
        debug_assert_finite!(transform.position);
        self.acceleration = self.apply_forces();
        debug_assert_finite!(self.acceleration);

        let i_inv = Mat3::from(transform.rotation)
            * inv_inertia_tensor
            * Mat3::from(transform.rotation).transposed();
        let angular_velocity = self.angular_velocity(i_inv);

        self.add_impulse((self.acceleration + self.acceleration) * (dt * 0.5));

        // apply rotation
        transform.rotation.rotate_x(angular_velocity.x * dt);
        transform.rotation.rotate_y(angular_velocity.y * dt);
        transform.rotation.rotate_z(angular_velocity.z * dt);

        self.is_active_time += dt;

        /*

        // apply acceleration
        self.velocity += self.acceleration * dt;
        self.angular_velocity += self.torque * dt;

        // apply rotation
        transform.rotation.rotate_x(self.angular_velocity.x * dt);
        transform.rotation.rotate_y(self.angular_velocity.y * dt);
        transform.rotation.rotate_z(self.angular_velocity.z * dt);

        // update position
        transform.position += self.velocity * dt;

        self.is_active_time += dt;*/
    }
}

pub fn update(
    is_paused: &mut bool,
    dt: f32,
    transforms: &mut Vec<Transform>,
    phx_objects: &mut Vec<PhysicsObject>,
) {
    let real_dt = dt;
    let phx_length = phx_objects.len();

    for i in 0..phx_length {
        phx_objects[i].rb.is_colliding_this_frame = false;

        // update last frame location
        phx_objects[i].rb.last_frame_location = transforms[i].position;

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
        // if phx_objects[i].rb.is_colliding_this_frame {
        //     println!("col >> {} = {}",i, phx_objects[i].rb.is_colliding_this_frame);
        //}
        phx_objects[i].rb.is_colliding = phx_objects[i].rb.is_colliding_this_frame;
    }
}
