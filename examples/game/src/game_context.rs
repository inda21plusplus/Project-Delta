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
const HIT_MAX_FORCE: f32 = 90.0;
const HIT_MIN_FORCE: f32 = 5.0;

#[derive(Debug)]
pub struct GameBall {
    pub _turn: u8,         // how many times played
    pub _player_index: u8, // what player is to play, start as 0
    pub _players: u8,      // how many players
    pub force: f32,
}

struct UnityTransform {
    position: Vec3,
    rotation: Vec3,
    scale: Vec3,
}

fn vec3_to_quaternion(rotation: Vec3) -> Quaternion {
    return to_quaternion(
        rotation.x.to_radians(),
        -rotation.y.to_radians(),
        rotation.z.to_radians(),
    );
}

//https://en.wikipedia.org/wiki/Conversion_between_quaternions_and_Euler_angles
fn to_quaternion(yaw: f32, pitch: f32, roll: f32) -> Quaternion // yaw (Z), pitch (Y), roll (X)
{
    // Abbreviations for the various angular functions
    let cy = f32::cos(yaw * 0.5);
    let sy = f32::sin(yaw * 0.5);
    let cp = f32::cos(pitch * 0.5);
    let sp = f32::sin(pitch * 0.5);
    let cr = f32::cos(roll * 0.5);
    let sr = f32::sin(roll * 0.5);

    return Quaternion::from_xyzw(
        cr * cp * cy + sr * sp * sy,
        sr * cp * cy - cr * sp * sy,
        cr * sp * cy + sr * cp * sy,
        cr * cp * sy - sr * sp * cy,
    );
}

impl GameScene {
    pub fn new(engine: &mut Engine) -> Result<Self, anyhow::Error> {
        let world = &mut engine.world;
        world.add_resource(HashMap::<VirtualKeyCode, bool>::new());

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
        let ball_material = PhysicsMaterial {
            friction: 1.0,
            restfullness: 1.0,
        };
        let mut rng = rand::thread_rng();

        let world = &mut engine.world;

        let table_material = PhysicsMaterial {
            friction: 1.0,
            restfullness: 1.0,
        };

        let ground_material = PhysicsMaterial {
            friction: 1.0,
            restfullness: 0.0,
        };

        world.add_resource(physics::Gravity::default());
        world.add_resource::<Vec<Line>>(vec![]);

        let unity_export: Vec<UnityTransform> = vec![
            UnityTransform {
                position: Vec3::new(0.0000, 0.0000, 0.0000),
                rotation: Vec3::new(0.0000, 0.0000, 0.0000),
                scale: Vec3::new(2.5400, 0.1000, 1.2700),
            },
            UnityTransform {
                position: Vec3::new(-1.4480, -0.1010, -0.8150),
                rotation: Vec3::new(0.0000, 45.0000, 0.0000),
                scale: Vec3::new(1.0500, 0.5000, 0.1000),
            },
            UnityTransform {
                position: Vec3::new(-1.4480, -0.1010, 0.8150),
                rotation: Vec3::new(0.0000, 315.0000, 0.0000),
                scale: Vec3::new(1.0500, 0.5000, 0.1000),
            },
            UnityTransform {
                position: Vec3::new(1.4480, -0.1010, 0.8150),
                rotation: Vec3::new(0.0000, 45.0000, 0.0000),
                scale: Vec3::new(1.0500, 0.5000, 0.1000),
            },
            UnityTransform {
                position: Vec3::new(1.4480, -0.1010, -0.8150),
                rotation: Vec3::new(0.0000, 315.0000, 0.0000),
                scale: Vec3::new(1.0500, 0.5000, 0.1000),
            },
            UnityTransform {
                position: Vec3::new(0.0000, -0.1010, -0.9000),
                rotation: Vec3::new(0.0000, 0.0000, 0.0000),
                scale: Vec3::new(1.0500, 0.5000, 0.1000),
            },
            UnityTransform {
                position: Vec3::new(0.0000, -0.1010, 0.9000),
                rotation: Vec3::new(0.0000, 0.0000, 0.0000),
                scale: Vec3::new(1.0500, 0.5000, 0.1000),
            },
            UnityTransform {
                position: Vec3::new(1.3200, -0.1010, 0.0000),
                rotation: Vec3::new(0.0000, 0.0000, 0.0000),
                scale: Vec3::new(0.1000, 0.5000, 1.0000),
            },
            UnityTransform {
                position: Vec3::new(-1.3200, -0.1010, 0.0000),
                rotation: Vec3::new(0.0000, 0.0000, 0.0000),
                scale: Vec3::new(0.1000, 0.5000, 1.0000),
            },
            UnityTransform {
                position: Vec3::new(-0.6200, -0.1010, 0.6900),
                rotation: Vec3::new(0.0000, 0.0000, 0.0000),
                scale: Vec3::new(1.0500, 0.5000, 0.1000),
            },
            UnityTransform {
                position: Vec3::new(0.6200, -0.1010, 0.6900),
                rotation: Vec3::new(0.0000, 0.0000, 0.0000),
                scale: Vec3::new(1.0500, 0.5000, 0.1000),
            },
            UnityTransform {
                position: Vec3::new(0.6200, -0.1010, -0.6900),
                rotation: Vec3::new(0.0000, 0.0000, 0.0000),
                scale: Vec3::new(1.0500, 0.5000, 0.1000),
            },
            UnityTransform {
                position: Vec3::new(-0.6200, -0.1010, -0.6900),
                rotation: Vec3::new(0.0000, 0.0000, 0.0000),
                scale: Vec3::new(1.0500, 0.5000, 0.1000),
            },
        ];

        for i in 0..unity_export.len() {
            let transform = &unity_export[i];
            let cube = world.spawn();
            world.add(
                cube,
                Transform {
                    position: transform.position * 10.0,
                    rotation: vec3_to_quaternion(transform.rotation),
                    scale: transform.scale * 5.0,
                },
            );
            world.add(cube, Rigidbody::new_static());

            world.add(
                cube,
                Collider::Cube(CubeCollider::new(
                    Vec3::one(),
                    if i == 0 {
                        ground_material
                    } else {
                        table_material
                    },
                )),
            );

            world.add(cube, table_model);
        }

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
                        Vec3::new(-5.0, 1.1 + BALL_RADIUS, 0.0)
                    } else {
                        Vec3::new(
                            diagonal_radius * (y as f32),
                            1.1 + BALL_RADIUS,
                            ((x as f32) - (max as f32) / 2.0) * (BALL_RADIUS * 2.0 + BALL_SPACING),
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
                        _turn: 0,
                        _player_index: 0,
                        _players: 2,
                        force: HIT_MAX_FORCE / 2.0,
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

        // phx drag
        query_iter!(engine.world, (rb: mut Rigidbody, transform : Transform) => {
            if rb.velocity().magnitude_squared() < MIN_SPEED * MIN_SPEED {
               rb.linear_momentum = Vec3::zero();
               rb.angular_momentum = Vec3::zero();
            } else {
                if transform.position.y > BALL_RADIUS {
                    let sub_max = 0.4*time.dt().as_secs_f32(); //* ;
                    let sub_of =sub_max*( rb.linear_momentum.magnitude().min(5.0) / rb.linear_momentum.magnitude());

                    rb.linear_momentum -= rb.linear_momentum * sub_of;
                    rb.angular_momentum -= rb.angular_momentum * sub_of;
                    rb.linear_momentum = Vec3::new(rb.linear_momentum.x,rb.linear_momentum.y.min(0.0),rb.linear_momentum.z).normalized() * rb.linear_momentum.magnitude();
                }
            }
        });
        let mut hit_line: Option<Line> = None;
        let mut force = 0.0;
        // raycast
        query_iter!(engine.world, (rb: mut Rigidbody, collider : Collider, transform : Transform, game: mut GameBall) => {
            if rb.velocity().magnitude_squared() <= MIN_SPEED*MIN_SPEED {
                let dt = time.dt().as_secs_f32();
                if is_key_down(VirtualKeyCode::Up) {
                    game.force += dt * HIT_MAX_FORCE;
                }
                if is_key_down(VirtualKeyCode::Down) {
                    game.force -= dt * HIT_MAX_FORCE;
                }
                game.force = game.force.max(HIT_MIN_FORCE).min(HIT_MAX_FORCE);
                force = game.force;

                if let Some(hit) = raycast(transform, &vec![*collider], Ray::new(camera.eye, (camera.target - camera.eye).normalized())) {
                    let direction_by_normal = Vec3::new(-hit.normal.x, 0.0, -hit.normal.z).normalized();
                    let direction_by_offset = Vec3::new(transform.position.x - camera.eye.x, 0.0, transform.position.z - camera.eye.z).normalized();
                    let direction = direction_by_normal * 1.0 + direction_by_offset * 0.0;

                    if is_key_down(VirtualKeyCode::Delete) || is_key_down(VirtualKeyCode::F) {
                        rb.add_impulse(direction * game.force);
                    } else {
                        hit_line = Some(Line { start: transform.position, end: transform.position + direction, color: Vec3::new(1.0,0.0,0.0) });
                    }
                }
            }
        });
        let mut lines: Vec<Line> = Vec::new();

        if let Some(line) = hit_line {
            let direction = (line.end - line.start).normalized();
            let mut min_distance = 10.0;
            query_iter!(engine.world, (collider : Collider, transform : Transform) => {
                if let Some(hit) = raycast(transform, &vec![*collider], Ray::new(line.start, direction)) {
                    if hit.distance < min_distance && hit.distance > BALL_RADIUS {
                        min_distance = hit.distance;
                    }
                }
            });

            lines.push(Line {
                start: line.start + direction * BALL_RADIUS,
                end: line.start + direction * min_distance,
                color: Vec3::new(1.0, 1.0, 1.0),
            });
            let force_scale = (force / HIT_MAX_FORCE);
            lines.push(Line {
                start: line.start + direction * BALL_RADIUS,
                end: line.start + direction * min_distance * force_scale,
                color: Vec3::new(force_scale, 1.0 - force_scale, 0.0),
            });
        }

        //query_iter!(engine.world, (collider : Collider, transform : Transform) => {

        //}

        query_iter!(engine.world, (rb: Rigidbody, transform : Transform) => {
            //lines.push(Line { start: transform.position, end: transform.position + rb.linear_momentum, color: Vec3::new(1.0,0.0,0.0) })
        });

        let lines_res = engine.world.resource_mut::<Vec<Line>>().unwrap();
        *lines_res = lines;

        let mut mgr = engine.renderer.get_models_mut();

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
