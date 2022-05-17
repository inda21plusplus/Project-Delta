use anyhow::Context as _;
use rand::Rng as _;

use common::{Quaternion, Transform, Vec3};
use game_engine::{
    ecs::query_iter,
    physics::{self, BoxCollider, Collider, PhysicsMaterial, Rigidbody, SphereCollider},
    rendering::{model::ModelIndex, Light, Line},
    Engine,
};

// TODO: remove
pub struct PhysicsScene {
    // TODO: move into world
    pub cube_model: ModelIndex,
    // TODO: move into world
    pub ball_model: ModelIndex,
    // TODO: move into world
    pub light: Light,
    // TODO: move into world
    pub extra_dt: f32,
}

impl PhysicsScene {
    pub fn new(engine: &mut Engine) -> Result<Self, anyhow::Error> {
        let mut rng = rand::thread_rng();

        let world = &mut engine.world;

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
            Collider::Box(BoxCollider::new(Vec3::one(), physics_material)),
        );

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
                    Collider::Box(BoxCollider::new(Vec3::one(), physics_material))
                } else {
                    Collider::Sphere(SphereCollider::new(1., physics_material))
                },
            );
        }

        Ok(Self {
            cube_model: engine
                .renderer
                .load_model("res/cube.obj")
                .with_context(|| "failed to open cube.obj")?,
            ball_model: engine
                .renderer
                .load_model("res/ball.obj")
                .with_context(|| "failed to open ball.obj")?,
            light: Light {
                pos: Vec3::unit_y() * 30.0,
                color: [1.0; 3],
                k_constant: 1.0,
                k_linear: 0.0014,
                k_quadratic: 0.000007,
            },
            extra_dt: 0.0,
        })
    }

    pub fn update(&mut self, engine: &mut Engine) {
        let mut mgr = engine.renderer.get_models_mut();
        let mut cube_transforms = vec![];
        let mut ball_transforms = vec![];
        query_iter!(engine.world, (transform: Transform, collider: Collider) => {
            match collider {
                Collider::Box(_) => &mut cube_transforms,
                Collider::Sphere(_) => &mut ball_transforms,
            }.push(*transform);
        });
        mgr.set_transforms(self.cube_model, cube_transforms);
        mgr.set_transforms(self.ball_model, ball_transforms);
    }
}
