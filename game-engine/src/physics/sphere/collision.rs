use std::{ops::Div, f32::{consts::PI, EPSILON}};

use crate::{
    physics::{
        collision::{pop_coliders, standard_collision},
        macros::{debug_assert_finite, squared},
        r#box::{get_closest_point, BoxColider},
        Mat3, RidgidBody, Vec3, proj,
    },
    renderer::Transform,
};

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
    dt: f32,
) {
    let re1 = c1.material.restfullness;
    let re2 = c2.material.restfullness;

    let gravity_vector = Vec3::new(0f32, -9.82, 0f32); //rb1.acceleration;
                                                       // println!("B: {} {}",rb1.is_colliding,rb2.is_colliding);
                                                       //let re2 = c2.material.restfullness;

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

    let alpha = (normal
        .dot(gravity_vector)
        .div(normal.magnitude() * gravity_vector.magnitude()))
    .acos();
    let acceleration_length = (gravity_vector * alpha.sin()).magnitude();
    let axis_of_acceleration = (normal.normalized() - (-gravity_vector).normalized()).normalized();
    let acceleration = axis_of_acceleration * acceleration_length;
    
    let real_v1 = proj(normal, rb1.acceleration);
    let slide = rb1.velocity - real_v1;

    //rb1.velocity -= slide;
    /*let vel_add = acceleration * dt + slide; // - friction stuff
    //rb1.velocity += vel_add-slide-slide;

    let o_div_2 = r*PI;
    let rad_per_sec =slide/o_div_2; 

    rb1.angular_velocity += normal.cross(acceleration).normalized()*rad_per_sec;*/
    //Spin angular velocity in rad per seconds around that axis (Quaternion::rotate_3d)

    
    let r = c1.get_radius(t1.scale);
    let i = 2.0 * rb1.mass * r * r / 5.0;

    let v_2 = rb2.velocity;
    let v_1 = rb1.velocity;

    let r_1 = point_of_contact - w1;
    let r_2 = point_of_contact - w2;

    let w_1 = rb1.angular_velocity;
    let w_2 = rb2.angular_velocity;

    let v_p1 = v_1 + w_1.cross(r_1);
    let v_p2 = v_2 + w_2.cross(r_2);

    let e = c1.material.restfullness; // dont do stuff
    let n_hat = -normal;

    let v_r = v_p2 - v_p1;
    let j_r_top = (-(1.0 + e) * v_r).dot(n_hat);

    let m_1 = rb1.mass;
    //let m_2 = rb2.mass;

    let I_1_inv = Mat3::new(1.0 / i, 0.0, 0.0, 0.0, 1.0 / i, 0.0, 0.0, 0.0, 1.0 / i);

    let something = (I_1_inv * r_1.cross(n_hat)).cross(r_1); //+ (I_2.inv() * r_2.cross(n_hat)).cross(r_2);

    let k = 1.0 / m_1 + something.dot(n_hat);
    let j = j_r_top/k;


    //rb1.angular_velocity = rb1.angular_velocity - dbg!(j_r*(I_1_inv*(r_1.cross(n_hat))));
    let angular_momentum = j * r_1.cross(normal)*dt;
    //rotation * inverseInertiaTensor * rotation^T
    let rotation = Mat3::from(t1.rotation);
    let a = rotation * I_1_inv * rotation.transposed();

    rb1.angular_velocity += a * angular_momentum;
    rb1.velocity += j * normal*dt;
    let velocity_at_point = rb1.velocity + rb1.angular_velocity.cross(r_1);
    let tangent_velocity = velocity_at_point - normal * velocity_at_point.dot(normal); 
    let epsilon = 0.001;
    if tangent_velocity.magnitude_squared() > epsilon*epsilon {
        let tangent = tangent_velocity.normalized();
        let vt = velocity_at_point.dot(tangent);
        let kt = (1.0/m_1) + Vec3::dot(Vec3::cross(r_1,tangent), a*Vec3::cross(r_1, tangent));
        let u = 0.5;
        let j = f32::clamp(j, 0.0, 1.0);

        let jt = f32::clamp( -vt / kt, -u * j, u * j );
        rb1.velocity += jt * tangent;
        rb1.angular_velocity += jt * Vec3::cross( r_1, tangent );
    } 
    //rb1.angular_velocity += j * cross( r, contact.normal );
    //let velocityAtPoint = v_1
    //let tangentVelocity = velocityAtPoint - contact.normal * dot( velocityAtPoint, contact.normal );

    //https://en.wikipedia.org/wiki/Collision_response#Impulse-based_friction_model
    /*let J = [
        i,0,0;
        0,i,0

    ]*/

    // rb1.angular_velocity

    //b1.velocity -= gravity_vector * dt;
    //rb1.velocity = Vec3::zero();

    pop_coliders(normal * overlap_distance, t1, t2, &rb1, &rb2);

    /*standard_collision(
        normal,
        rb1,
        rb2,
        point_of_contact - w1,
        point_of_contact - w2,
        re1,
        re2,
    );*/
    /*
    let real_v1 = crate::physics::proj(normal, rb1.velocity);
    if rb2.is_static && rb1.is_colliding {
        let accel = (rb1.velocity - real_v1);// * (1.0 - c1.material.friction);
        rb1.velocity += accel;
    }*/
}
