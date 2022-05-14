use anyhow::Context as _;
use rand::Rng as _;

use common::{Quaternion, Transform, Vec3};
use game_engine::{
    physics::{
        self, BoxCollider, Collider, PhysicsMaterial, PhysicsObject, Rigidbody, SphereCollider,
    },
    rendering::{model::ModelIndex, Light},
    Context,
};

pub struct PhysicsScene {
    pub cube_model: ModelIndex,
    pub ball_model: ModelIndex,
    pub transforms: Vec<Transform>,
    pub n_cubes: usize,
    pub objects: Vec<PhysicsObject>,
    pub light: Light,
    pub extra_dt: f32,
}

impl PhysicsScene {
    pub fn new(context: &mut Context) -> Result<Self, anyhow::Error> {
        let mut instances = vec![Transform {
            position: Vec3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::rotation_x(10.0f32.to_radians()),
            scale: Vec3::new(10.0, 1.0, 10.0),
        }];
        let cubes = 20;
        let spheres = 20;
        let mut rng = rand::thread_rng();
        for _ in 0..(cubes + spheres) {
            let scale = rng.gen_range(1.0..1.5);
            instances.push(Transform {
                position: Vec3::new(
                    rng.gen_range(-10.0..10.0),
                    rng.gen_range(14.0..30.0),
                    rng.gen_range(-10.0..10.0),
                ),
                rotation: Quaternion::identity()
                    .rotated_x(rng.gen_range(0.0f32..360.0f32).to_radians())
                    .rotated_y(rng.gen_range(0.0f32..360.0f32).to_radians())
                    .rotated_z(rng.gen_range(0.0f32..360.0f32).to_radians()),
                scale: Vec3::new(scale, scale, scale),
            })
        }

        let physics_material = PhysicsMaterial {
            friction: 1.0,
            restfullness: 0.0,
        };

        let mut obj1 = PhysicsObject::new(
            Rigidbody::new(1.0),
            Collider::Box(BoxCollider::new(Vec3::new(1.0, 1.0, 1.0), physics_material)),
        );
        obj1.rb.is_static = true;

        let mut physics_objects: Vec<PhysicsObject> = vec![obj1];

        for _ in 0..cubes {
            physics_objects.push(PhysicsObject::new(
                Rigidbody::new(1.0),
                Collider::Box(BoxCollider::new(Vec3::new(1.0, 1.0, 1.0), physics_material)),
            ));
        }

        for _ in 0..spheres {
            physics_objects.push(PhysicsObject::new(
                Rigidbody::new(1.0),
                Collider::Sphere(SphereCollider::new(1.0, physics_material)),
            ));
        }

        Ok(Self {
            cube_model: context
                .renderer
                .load_model("res/cube.obj")
                .with_context(|| "failed to open cube.obj")?,
            ball_model: context
                .renderer
                .load_model("res/ball.obj")
                .with_context(|| "failed to open ball.obj")?,
            n_cubes: cubes,
            transforms: instances,
            objects: physics_objects,
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

    pub fn update(&mut self, dt: f32, ctx: &mut Context) {
        const FIXED_DT: f32 = 0.02f32;
        self.extra_dt += dt;
        while self.extra_dt > FIXED_DT {
            physics::update(FIXED_DT, &mut self.transforms, &mut self.objects);
            self.extra_dt -= FIXED_DT;
        }

        let mut mgr = ctx.renderer.get_models_mut();
        mgr.set_transforms(
            self.cube_model,
            self.transforms[..self.n_cubes + 1].to_owned(),
        );
        mgr.set_transforms(
            self.ball_model,
            self.transforms[self.n_cubes + 1..].to_owned(),
        );
    }
}
