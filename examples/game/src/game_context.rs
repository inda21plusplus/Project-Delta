use std::collections::HashMap;

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
pub struct GameScene {
    pub ball_models: Vec<ModelIndex>,
    // TODO: move into world
    pub cube_model: ModelIndex,
    // TODO: move into world
    pub ball_model: ModelIndex,
    // TODO: move into world
    pub light: Light,
    // TODO: move into world
    pub extra_dt: f32,
}
const BALL_COUNT: usize = 15;
const BALL_RADIUS: f32 = 0.5715;
const BALL_SPACING: f32 = 0.00;
const BALL_MASS: f32 = 1.70;
const MIN_SPEED: f32 = 0.5;

pub struct GameBall {
    pub turn: u8,         // how many times played
    pub player_index: u8, // what player is to play, start as 0
    pub players: u8,      // how many players
}

impl GameScene {
    pub fn new(engine: &mut Engine) -> Result<Self, anyhow::Error> {
        // models
        let table_model = engine
            .renderer
            .load_model("res/cube.obj")
            .with_context(|| "failed to open cube.obj")?;

        let mut ball_models = Vec::<ModelIndex>::with_capacity(BALL_COUNT + 1);
        for i in 0..=BALL_COUNT {
            println!("Opening {}", i);
            ball_models.push(
                engine
                    .renderer
                    .load_model(format!("res/balls/{}.obj", i))
                    .with_context(|| "failed to open ball obj")?,
            );
        }

        let mut rng = rand::thread_rng();

        let world = &mut engine.world;

        let table_material = PhysicsMaterial {
            friction: 1.0,
            restfullness: 0.0,
        };

        let ball_material = PhysicsMaterial {
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
                rotation: Quaternion::rotation_x(0.0f32.to_radians()),
                scale: Vec3::new(10.0, 1.0, 10.0),
            },
        );
        world.add(platform, Rigidbody::new_static());
        world.add(
            platform,
            Collider::Cube(CubeCollider::new(Vec3::one(), table_material)),
        );
        world.add(platform, table_model);

        let mut x = -1;
        let mut y = 0;
        let mut max = 0;
        let diagonal_radius = BALL_SPACING + BALL_RADIUS * 2.0 / 2.0f32.sqrt();
        for i in 0..=BALL_COUNT {
            let ball = world.spawn();
            world.add(
                ball,
                Transform {
                    position: if i == 0 {
                        Vec3::new(0.0, 3.0, -5.0)
                    } else {
                        Vec3::new(
                            ((x as f32) - (max as f32) / 2.0) * (BALL_RADIUS * 2.0 + BALL_SPACING),
                            3.0,
                            diagonal_radius * (y as f32),
                        )
                    },
                    rotation: Quaternion::identity()
                        .rotated_x(rng.gen_range(0.0f32..360.0f32).to_radians())
                        .rotated_y(rng.gen_range(0.0f32..360.0f32).to_radians())
                        .rotated_z(rng.gen_range(0.0f32..360.0f32).to_radians()), //Quaternion::rotation_y(90.0f32.to_radians()).rotated_z(-90.0f32.to_degrees()),//.rotated_x(90.0f32.to_degrees()),
                    scale: Vec3::new(BALL_RADIUS, BALL_RADIUS, BALL_RADIUS),
                },
            );
            world.add(ball, Rigidbody::new(BALL_MASS));
            
            if i == 0 {
                println!("Added gameball");
                world.add(
                    ball,
                    GameBall {
                        turn: 0,
                        player_index: 0,
                        players: 2,
                    },
                );
            }
            
            world.add(
                ball,
                Collider::Sphere(SphereCollider::new(1.0, ball_material)),
            );

            world.add(ball, ball_models[i]);


            // ugly ik, but I dont want to solve a leet problem rn
            x += 1;
            if x > max {
                x = 0;
                max += 1;
                y += 1;
            }
        }

        Ok(Self {
            ball_models,
            cube_model: table_model,
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
        let mut transforms: HashMap<ModelIndex, Vec<Transform>> = HashMap::new();

        query_iter!(engine.world, (game : GameBall) => {
            //if rb.velocity().magnitude_squared() < MIN_SPEED * MIN_SPEED {
                println!("can play");
               // rb.linear_momentum = Vec3::zero();
            //}
        });

        query_iter!(engine.world, (transform: Transform, id: ModelIndex) => {
            match transforms.get_mut(id) {
                Some(items) => {
                    items.push(*transform);
                },
                None => {
                    transforms.insert(*id,vec![*transform]);
                },
            };
        });

        mgr.set_all_transforms(transforms);
    }
}
