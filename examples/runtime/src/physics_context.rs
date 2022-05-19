use anyhow::Context as _;
use rand::Rng as _;

use common::{Quaternion, Transform, Vec3};
use game_engine::{
    physics::{self, Collider, CubeCollider, PhysicsMaterial, Rigidbody, SphereCollider},
    rendering::{Light, Line, WorldId},
    Engine,
};

pub fn setup_scene(engine: &mut Engine) -> Result<(), anyhow::Error> {
    let mut rng = rand::thread_rng();

    let world = &mut engine.world;

    let world_id = *world.resource::<WorldId>().unwrap();

    let cube_model = engine
        .renderer
        .load_model(world_id, "res/cube.obj")
        .with_context(|| "failed to open cube.obj")?;
    let ball_model = engine
        .renderer
        .load_model(world_id, "res/ball.obj")
        .with_context(|| "failed to open ball.obj")?;

    let physics_material = PhysicsMaterial {
        friction: 1.0,
        restfullness: 0.0,
    };

    world.add_resource(physics::Gravity::default());
    world.add_resource::<Vec<Line>>(vec![]);

    let platform = world.spawn();
    world.add(
        platform,
        Transform {
            position: Vec3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::rotation_x(10.0f32.to_radians()),
            scale: Vec3::new(10.0, 1.0, 10.0),
        },
    );
    world.add(platform, Rigidbody::new_static());
    world.add(
        platform,
        Collider::Cube(CubeCollider::new(Vec3::one(), physics_material)),
    );
    world.add(platform, cube_model);

    for i in 0..40 {
        let entity = world.spawn();
        let scale = rng.gen_range(1.0..1.5);
        world.add(
            entity,
            Transform {
                position: Vec3::new(
                    rng.gen_range(-10.0..10.0),
                    rng.gen_range(14.0..30.0),
                    rng.gen_range(-10.0..10.0),
                ),
                rotation: Quaternion::identity()
                    .rotated_x(rng.gen_range(0.0f32..360.0f32).to_radians())
                    .rotated_y(rng.gen_range(0.0f32..360.0f32).to_radians())
                    .rotated_z(rng.gen_range(0.0f32..360.0f32).to_radians()),
                scale: Vec3::broadcast(scale),
            },
        );
        world.add(entity, Rigidbody::new(1.));
        world.add(
            entity,
            if i < 20 {
                Collider::Cube(CubeCollider::new(Vec3::one(), physics_material))
            } else {
                Collider::Sphere(SphereCollider::new(1., physics_material))
            },
        );
        world.add(entity, if i < 20 { cube_model } else { ball_model });
    }

    let light = world.spawn();
    world.add(light, Transform::at(Vec3::unit_y() * 30.));
    world.add(
        light,
        Light {
            color: [1.0; 3],
            k_constant: 1.0,
            k_linear: 0.0014,
            k_quadratic: 0.000007,
        },
    );

    Ok(())
}
