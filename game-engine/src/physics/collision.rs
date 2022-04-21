use std::collections::HashMap;

use super::{
    get_position,
    r#box::{collision::is_colliding_box_vs_box, BoxColider},
    sphere::collision::{is_colliding_sphere_vs_box, is_colliding_sphere_vs_sphere},
    Collider, PhysicsObject, RidgidBody, Vec3,
};
use crate::physics::sphere::collision::collide_sphere_vs_sphere;
use crate::{
    physics::{
        is_finite, macros::debug_assert_finite, proj, r#box::collision::collide_box_vs_box,
        sphere::collision::collide_sphere_vs_box,
    },
    renderer::Transform,
};

/// Returns true if 2 objects are colliding
#[must_use]
pub fn is_colliding(c1: &Collider, t1: &Transform, c2: &Collider, t2: &Transform) -> bool {
    let w1 = get_position(t1, c1);
    let w2 = get_position(t2, c2);

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

pub fn standard_collision(
    normal: Vec3,
    rb1: &mut RidgidBody,
    rb2: &mut RidgidBody,
    // offset from point of contact
    o1: Vec3,
    o2: Vec3,
    // not used atm, restfullness
    _re1: f32,
    _re2: f32,
) {
    debug_assert_finite!(normal);

    let v1 = rb1.velocity;
    let v2 = rb2.velocity;

    let m1 = rb1.mass;
    let m2 = rb2.mass;

    debug_assert!(m1 > 0.0);
    debug_assert!(m2 > 0.0);

    // proj the velocities on the normal, this way you can move the frame of
    // refrence and think of the two objects are coliding head on
    let real_v1 = proj(normal, v1);
    let real_v2 = proj(normal, v2);

    let bouncy_ness = 0.6;
    let friction = 0.7;
    if rb1.is_static {
        rb2.velocity = (v2 - (1.0 + bouncy_ness) * real_v2) * friction;
        return;
    } else if rb2.is_static {
        rb1.velocity = (v1 - (1.0 + bouncy_ness) * real_v1) * friction;
        return;
    }

    // using a perfectly elastic collision on each axis
    let (new_v1, new_v2) = standard_elastic_collision_3(m1, &real_v1, m2, &real_v2);

    // inital velocity - velocity used to colide "head on" + velocity after coliding "head on"
    rb1.velocity = v1 - real_v1;
    rb2.velocity = v2 - real_v2;

    rb1.add_impulse_at_location(new_v1, o1);
    rb2.add_impulse_at_location(new_v2, o2);
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
            Collider::BoxColider(b2) => collide_sphere_vs_box(b1, rb1, t1, w1, b2, rb2, t2, w2),
        },
        Collider::BoxColider(b1) => match c2 {
            Collider::SphereColider(b2) => collide_sphere_vs_box(b2, rb2, t2, w2, b1, rb1, t1, w1),
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
                //println!("Colliding!");
                has_colided = true;
                debug_assert_finite!(rb1.velocity);
                debug_assert_finite!(rb2.velocity);
                debug_assert_finite!(rb1.angular_velocity);
                debug_assert_finite!(rb2.angular_velocity);

                rb1.is_active = true;
                rb2.is_active = true;
                rb1.is_active_time = 0.0;
                rb2.is_active_time = 0.0;
                //solve_colliding(c1, rb1, t1, c2, rb2, t2);
                match (c1, c2) {
                    (Collider::BoxColider(_), Collider::BoxColider(_)) => {
                        current_post_fix.push(c2_index);
                    }
                    _ => {
                        solve_colliding(c1, rb1, t1, c2, rb2, t2);
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
            }
            else {
                is_touching = search_location;
            }
            search_location += search_length * if colliding { -1.0 } else { 1.0 };
        }
  
        t1.position = t1_post * is_touching + t1_pre * (1.0 - is_touching);
        t2.position = t2_post * is_touching + t2_pre * (1.0 - is_touching);

        for c2_index in &post_fix[c1_index] {
            let c2 = &cc2[*c2_index];

            solve_colliding(c1, rb1, t1, c2, rb2, t2);
        }
        t1.position = t1_post * is_not_touching + t1_pre * (1.0 - is_not_touching);
        t2.position = t2_post * is_not_touching + t2_pre * (1.0 - is_not_touching);
    }

    has_colided
}

impl RidgidBody {
    pub fn add_impulse(&mut self, force: Vec3) {
        self.velocity += force / self.mass;
    }

    pub fn add_impulse_at_location(&mut self, velocity: Vec3, location: Vec3) {
        debug_assert_finite!(velocity);
        debug_assert_finite!(location);

        //debug_assert!(velocity.magnitude_squared() != 0.0, "velocity is too close to 0 = {}", velocity);

        // if zero velocity is applied then nothing happends
        if velocity.magnitude_squared() == 0.0 {
            return;
        }

        // Bullet Block Explained! https://youtu.be/BLYoyLcdGPc no velocity is lost due to angular velocity irl,
        // so it is not removed here
        self.velocity += velocity;

        //https://en.wikipedia.org/wiki/Angular_velocity

        // just random shit
        return;
        let offset = self.center_of_mass_offset + location;
        let normal = offset;

        let rotation_around = -(normal.normalized().cross(velocity.normalized())).normalized();
        debug_assert!(
            is_finite(&rotation_around),
            "rotation_around = {} normal {} velocity {}",
            rotation_around,
            normal,
            velocity
        );

        let torque = rotation_around * 10.0; //velocity *  / offset.magnitude();

        self.angular_velocity += torque;

        // TODO idk what angular_velocity is
    }

    pub fn step(&mut self, dt: f32, transform: &mut Transform) {
        if self.is_static {
            return;
        }

        // apply acceleration
        self.velocity += self.acceleration * dt;
        self.angular_velocity += self.torque * dt;

        // apply rotation
        transform.rotation.rotate_x(self.angular_velocity.x * dt);
        transform.rotation.rotate_y(self.angular_velocity.y * dt);
        transform.rotation.rotate_z(self.angular_velocity.z * dt);

        // update position
        transform.position += self.velocity * dt;

        self.is_active_time += dt;
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
        let (phx_first, phx_last) = phx_objects.split_at_mut(i + 1);
        //if phx_first[i].rb.is_static || !phx_first[i].rb.is_active {
        //    continue; // we dont care about non active or static objects
        //}

        let (trans_first, trans_last) = transforms.split_at_mut(i + 1);

        // update last frame location
        phx_first[i].rb.last_frame_location = trans_first[i].position;

        // simulate one step in the simulation
        phx_first[i].rb.step(real_dt, &mut trans_first[i]);

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
            ) {
                has_colided = true;
            }
        }

        if has_colided {
            *is_paused = true;
        }
    }
}
