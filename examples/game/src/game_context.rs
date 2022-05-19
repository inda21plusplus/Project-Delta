use std::collections::HashMap;

use anyhow::Context as _;
use rand::Rng as _;

use common::{Quaternion, Ray, Transform, Vec3};
use game_engine::{
    ecs::query_iter,
    physics::{
        self, raycast::raycast, Collider, CubeCollider, PhysicsMaterial, Rigidbody, SphereCollider,
    },
    rendering::{model::ModelIndex, Light, Line},
    Engine, Time,
};
use winit::event::VirtualKeyCode;

// TODO: remove
pub struct GameScene {
    pub ball_models: Vec<ModelIndex>,
    // TODO: move into world
    pub cube_model: ModelIndex,
    // TODO: move into world
    pub ball_model: ModelIndex,
    // TODO: move into world
    pub lights: Vec<Light>,
    // TODO: move into world
    pub extra_dt: f32,
}
const BALL_COUNT: usize = 15;
const BALL_RADIUS: f32 = 0.5715;
const BALL_SPACING: f32 = 0.00;
const BALL_MASS: f32 = 1.70;
const MIN_SPEED: f32 = 0.2;
const HIT_FORCE: f32 = 90.0;

#[derive(Debug)]
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
            restfullness: -2.0,
        };

        let ball_material = PhysicsMaterial {
            friction: 1.0,
            restfullness: 1.5,
        };

        world.add_resource(HashMap::<VirtualKeyCode, bool>::new());

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
                        Vec3::new(0.0, 1.0 + BALL_RADIUS, -5.0)
                    } else {
                        Vec3::new(
                            ((x as f32) - (max as f32) / 2.0) * (BALL_RADIUS * 2.0 + BALL_SPACING),
                            1.0 + BALL_RADIUS,
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

        let mut lights: Vec<Light> = Vec::new();
        for x in [-1.0, 1.0] {
            for z in [-1.0, 1.0] {
                lights.push(Light {
                    pos: Vec3::new(x * 4.0, 3.0, z * 4.0),
                    color: [1.0; 3],
                    k_constant: 1.0,
                    k_linear: 0.14,
                    k_quadratic: 0.07,
                })
            }
        }

        Ok(Self {
            ball_models,
            cube_model: table_model,
            ball_model: engine
                .renderer
                .load_model("res/ball.obj")
                .with_context(|| "failed to open ball.obj")?,
            lights,
            extra_dt: 0.0,
        })
    }

    pub fn update(&mut self, engine: &mut Engine) {
        let mut transforms: HashMap<ModelIndex, Vec<Transform>> = HashMap::new();
        let camera = engine.renderer.camera;
        let key_map = engine
            .world
            .resource::<HashMap<VirtualKeyCode, bool>>()
            .unwrap();

        let time = engine.world.resource::<Time>().unwrap();

        let is_key_down = |keycode: VirtualKeyCode| -> bool {
            if let Some(is_down) = key_map.get(&keycode) {
                return *is_down;
            } else {
                false
            }
        };

        query_iter!(engine.world, (rb: mut Rigidbody, collider : Collider, transform : Transform, game: GameBall) => {
            if rb.velocity().magnitude_squared() < MIN_SPEED * MIN_SPEED {
                rb.linear_momentum = Vec3::zero();
                if is_key_down(VirtualKeyCode::Delete) {
                    if let Some(hit) = raycast(transform, &vec![*collider], Ray::new(camera.eye,camera.target - camera.eye)) {
                        //let direction = Vec3::new(-hit.normal.x, 0.0, -hit.normal.z).normalized();
                        let direction =Vec3::new(transform.position.x - camera.eye.x, 0.0, transform.position.z - camera.eye.z).normalized();

                        println!("Hit at {:?}",direction);
                        rb.add_impulse(direction * HIT_FORCE);
                    }
                }
            }
        });

        let mut mgr = engine.renderer.get_models_mut();
        query_iter!(engine.world, (rb: mut Rigidbody) => {
            let sub = 0.05*time.dt().as_secs_f32();
            rb.linear_momentum -= rb.linear_momentum*sub;
            rb.angular_momentum -= rb.angular_momentum*sub; 
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
