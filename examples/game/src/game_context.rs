use common::{Quaternion, Transform, Vec3};

use game_engine::{
    physics::{
        collision::update as physics_update, r#box::BoxColider, sphere::SphereColider, Collider,
        PhysicsMaterial, PhysicsObject, RidgidBody,
    },
    rendering::{model::ModelIndex, Light},
    Context,
};

use anyhow::Context as _;

use rand::{prelude::SliceRandom, Rng};

pub struct GameScene {
    // where index 0 is ball 1
    pub ball_models: Vec<ModelIndex>,
    pub table_model: ModelIndex,
    pub cube_model: ModelIndex,

    pub transforms: Vec<Transform>,
    pub objects: Vec<PhysicsObject>,
    pub lights: Vec<Light>,
    pub extra_dt: f32,
    pub total_time: f32,
}

const BALL_COUNT: usize = 15;
const BALL_RADIUS: f32 = 0.5715;
const BALL_SPACING: f32 = 0.00;
const BALL_MASS: f32 = 1.70;
const GRAVITY: f32 = -9.82;

impl GameScene {
    pub fn new(context: &mut Context) -> Result<Self, anyhow::Error> {
        let mut instances = Vec::new();

        let physics_material = PhysicsMaterial {
            friction: 1.0,
            restfullness: 2.0,
        };

        let table_material = PhysicsMaterial {
            friction: 1.0,
            restfullness: -2.5,
        };

        let gravity = Vec3::new(0.0, GRAVITY, 0.0);

        let mut table = PhysicsObject::new(
            RidgidBody::new(Vec3::zero(), 1.0),
            Collider::BoxColider(BoxColider::new(Vec3::new(1.0, 1.0, 1.0), table_material)),
        );
        table.rb.is_static = true;

        let mut physics_objects: Vec<PhysicsObject> = Vec::new(); //obj3, obj4 vec![obj1.clone(); 16];

        let mut x = 0;
        let mut y = 0;
        let mut max = 0;
        let diagonal_radius = BALL_SPACING + BALL_RADIUS * 2.0 / 2.0f32.sqrt();

        let mut rng = rand::thread_rng();
        for _ in 0..=BALL_COUNT {
            physics_objects.push(PhysicsObject::new(
                RidgidBody::new(gravity, BALL_MASS),
                Collider::SphereColider(SphereColider::new(1.0, physics_material)),
            ));
        }

        for _ in 0..BALL_COUNT {
            instances.push(Transform {
                position: Vec3::new(
                    ((x as f32) - (max as f32) / 2.0) * (BALL_RADIUS * 2.0 + BALL_SPACING),
                    10.0,
                    diagonal_radius * (y as f32),
                ),
                rotation: Quaternion::identity()
                    .rotated_x(rng.gen_range(0.0f32..360.0f32).to_radians())
                    .rotated_y(rng.gen_range(0.0f32..360.0f32).to_radians())
                    .rotated_z(rng.gen_range(0.0f32..360.0f32).to_radians()), //Quaternion::rotation_y(90.0f32.to_radians()).rotated_z(-90.0f32.to_degrees()),//.rotated_x(90.0f32.to_degrees()),
                scale: Vec3::new(BALL_RADIUS, BALL_RADIUS, BALL_RADIUS),
            });

            // ugly ik, but I dont want to solve a leet problem rn
            x += 1;
            if x > max {
                x = 0;
                max += 1;
                y += 1;
            }
        }
        instances.shuffle(&mut rng);

        // white ball
        instances.push(Transform {
            position: Vec3::new(0.0, 10.0, -5.0),
            rotation: Quaternion::identity(),
            scale: Vec3::new(BALL_RADIUS, BALL_RADIUS, BALL_RADIUS),
        });

        let len = instances.len();
        instances.swap(0, len - 1);

        instances.push(Transform {
            position: Vec3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::rotation_x(0.0f32.to_radians()),
            scale: Vec3::new(10.0, 1.0, 10.0),
        });

        physics_objects.push(table);

        let mut ball_models = Vec::<ModelIndex>::with_capacity(BALL_COUNT + 1);
        for i in 0..=BALL_COUNT {
            println!("Opening {}", i);
            ball_models.push(
                context
                    .renderer
                    .load_model(format!("res/balls/{}.obj", i))
                    .with_context(|| "failed to open ball obj")?,
            );
        }

        let table = context
            .renderer
            .load_model("res/cube.obj")
            .with_context(|| "failed to open cube.obj")?;

        let cube = context
            .renderer
            .load_model("res/cube.obj")
            .with_context(|| "failed to open cube.obj")?;

        println!("Finished init");
        Ok(Self {
            total_time: 0.0f32,
            ball_models,
            table_model: table,
            transforms: instances,
            objects: physics_objects,
            lights: Self::create_lights(),
            extra_dt: 0.0,
            cube_model: cube,
        })
    }

    pub fn update(&mut self, dt: f32, ctx: &mut Context) {
        let mut pause = false;
        self.total_time += dt;

        const PHX_STEP: f32 = 0.02f32;
        self.extra_dt += dt;
        if self.total_time > 3.0 {
            self.objects[0].rb.add_force(Vec3::new(0.0, 0.0, 90.0));
            self.total_time = 0.0;
        }

        while self.extra_dt > PHX_STEP {
            physics_update(
                &mut pause,
                PHX_STEP,
                &mut self.transforms,
                &mut self.objects,
            );
            self.extra_dt -= PHX_STEP;
        }

        let mut mgr = ctx.renderer.get_models_mut();
        for i in 0..=BALL_COUNT {
            mgr.set_transforms(self.ball_models[i], vec![self.transforms[i]].to_owned())
        }

        /*
        mgr.set_transforms(
            self.table_model,
            self.transforms[..self.n_cubes + 1].to_owned(),
        );*/
        mgr.set_transforms(
            self.cube_model,
            self.transforms[BALL_COUNT + 1..].to_owned(),
        );
    }

    pub fn create_lights() -> Vec<Light> {
        vec![Light {
            pos: Vec3::unit_y() * 30.0,
            color: [1.0; 3],
            k_constant: 1.0,
            k_linear: 0.0014,
            k_quadratic: 0.000007,
        }]
    }
}
