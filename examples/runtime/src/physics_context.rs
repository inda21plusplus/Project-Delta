use anyhow::Context as _;
use rand::Rng as _;

use common::{Quaternion, Transform, Vec3};
use game_engine::{
    ecs::query_iter,
    physics::{self, Collider, CubeCollider, PhysicsMaterial, Rigidbody, SphereCollider},
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
            restfullness: 1.0,
        };

        let ground_material = PhysicsMaterial {
            friction: 1.0,
            restfullness: 0.0,
        };

        world.add_resource(physics::Gravity::zero());
        world.add_resource::<Vec<Line>>(vec![]);
        /*
        let platform = world.spawn();
        world.add(
            platform,
            Transform {
                position: Vec3::new(0.0, 0.0, 0.0),
                rotation: Quaternion::rotation_x(0.0f32.to_radians()),
                scale: Vec3::new(40.0, 1.0, 10.0),
            },
        );
        world.add(platform, Rigidbody::new_static());
        world.add(
            platform,
            Collider::Cube(CubeCollider::new(Vec3::one(), ground_material)),
        );*/

        let platform2 = world.spawn();
        world.add(
            platform2,
            Transform {
                position: Vec3::new(30.0, 0.0, 0.0),
                rotation: Quaternion::rotation_y(0.0f32.to_radians()),
                scale: Vec3::new(1.0, 30.0, 30.0),
            },
        );
        world.add(platform2, Rigidbody::new_static());
        world.add(
            platform2,
            Collider::Cube(CubeCollider::new(Vec3::one(), physics_material)),
        );

        for i in 0..1 {
            let entity = world.spawn();
            let scale = rng.gen_range(1.0..1.5);
            world.add(
                entity,
                Transform {
                    position: Vec3::new(
                        rng.gen_range(-10.0..10.0),
                        2.0, //rng.gen_range(14.0..30.0),
                        rng.gen_range(-10.0..10.0),
                    ),
                    rotation: Quaternion::identity()
                        .rotated_x(rng.gen_range(0.0f32..360.0f32).to_radians())
                        .rotated_y(rng.gen_range(0.0f32..360.0f32).to_radians())
                        .rotated_z(rng.gen_range(0.0f32..360.0f32).to_radians()),
                    scale: Vec3::broadcast(scale),
                },
            );
            let mut rb = Rigidbody::new(0.5);
            rb.add_impulse(Vec3::new(10.0, 0.0, 0.0));
            world.add(entity, rb);
            world.add(
                entity,
                if i < 0 {
                    Collider::Cube(CubeCollider::new(Vec3::one(), physics_material))
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
                Collider::Cube(_) => &mut cube_transforms,
                Collider::Sphere(_) => &mut ball_transforms,
            }.push(*transform);
        });
        let mut lines: Vec<Line> = Vec::new();
        query_iter!(engine.world, (rb: Rigidbody, transform : Transform) => {
            println!("MAG::: {}",rb.linear_momentum.magnitude());
            lines.push(Line { start: transform.position, end: transform.position + rb.linear_momentum, color: Vec3::new(1.0,0.0,0.0) })
        });

        let lines_res = engine.world.resource_mut::<Vec<Line>>().unwrap();
        *lines_res = lines;

        mgr.set_transforms(self.cube_model, cube_transforms);
        mgr.set_transforms(self.ball_model, ball_transforms);
    }
}
