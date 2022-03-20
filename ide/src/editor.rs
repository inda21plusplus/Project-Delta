use std::{ops::ControlFlow, time::Instant};

use common::{Quaternion, Transform, Vec2, Vec3};
use game_engine::{
    rendering::{model::ModelIndex, Renderer},
    Context,
};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
};

use crate::{
    camera_controller::CameraController,
    window::{Window, WindowMode},
};

pub struct Editor {
    window: Window,
    context: Context,
    camera_controller: CameraController,
    scene: ExampleScene,
    last_frame: Instant,
}

// TODO: Things in here should exist in the ECS
pub struct ExampleScene {
    start_time: Instant,
    cube_model: ModelIndex,
    ball_model: ModelIndex,
    transforms: Vec<Transform>,
}

impl Editor {
    pub fn new() -> anyhow::Result<(EventLoop<()>, Self)> {
        let event_loop = EventLoop::new();

        let icon = image::open("res/icon.png")?.into_rgba8();
        let (icon_width, icon_height) = icon.dimensions();
        let icon = winit::window::Icon::from_rgba(icon.into_raw(), icon_width, icon_height)?;

        let window = Window::new(&event_loop, Some(icon))?;
        let mut context = Context {
            renderer: Renderer::new(
                window.raw_window_handle(),
                window.inner_size(),
                [0.229, 0.507, 0.921],
            )?,
        };

        let camera_controller = CameraController::new(
            10.0,
            0.1,
            Vec3::new(-16.0, 4.0, 1.0),
            Vec2::new(-0.3, 135f32.to_radians()),
        );

        let scene = ExampleScene::new(&mut context)?;
        Ok((
            event_loop,
            Self {
                window,
                context,
                camera_controller,
                scene,
                last_frame: Instant::now(),
            },
        ))
    }

    pub fn handle_event(&mut self, event: Event<()>) -> ControlFlow<()> {
        match event {
            Event::DeviceEvent { event, .. }
                if self.window.window_mode() == WindowMode::CameraMode =>
            {
                self.camera_controller.process_device_events(&event);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == self.window.winit_window().id() => return ControlFlow::Break(()),
            Event::WindowEvent {
                event: WindowEvent::Resized(winit::dpi::PhysicalSize { width, height }),
                window_id,
            } if window_id == self.window.winit_window().id() => {
                self.context.renderer.resize((width, height));
                self.window.update_size();
            }
            Event::WindowEvent { event, .. } => {
                self.camera_controller.process_window_events(&event);
                if let WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(keycode),
                            state,
                            ..
                        },
                    ..
                } = event
                {
                    self.handle_keyboard_input(keycode, state);
                }
            }
            Event::MainEventsCleared => self.window.winit_window().request_redraw(),
            Event::RedrawRequested(_) => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_frame).as_secs_f32();
                self.last_frame = now;
                self.update(dt);
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn handle_keyboard_input(&mut self, keycode: VirtualKeyCode, state: ElementState) {
        if state != ElementState::Pressed {
            return;
        }
        match keycode {
            VirtualKeyCode::Q => {
                self.window
                    .set_window_mode(WindowMode::CursorMode)
                    .unwrap_or_else(|_| log::error!("Could not unlock cursor"));
            }
            VirtualKeyCode::E => {
                self.window
                    .set_window_mode(WindowMode::CameraMode)
                    .unwrap_or_else(|_| log::error!("Could not lock cursor"));
            }
            _ => {}
        }
    }

    fn update(&mut self, dt: f32) {
        self.scene.update(dt, &mut self.context);
        self.camera_controller
            .update_camera(dt, &mut self.context.renderer.camera);
        self.context.renderer.update_camera();

        self.context
            .renderer
            .render()
            .unwrap_or_else(|err| log::error!("Failed to render: {}", err))
    }
}

impl ExampleScene {
    pub fn new(context: &mut Context) -> anyhow::Result<Self> {
        Ok(Self {
            start_time: Instant::now(),
            cube_model: context.renderer.load_model("res/cube.obj")?,
            ball_model: context.renderer.load_model("res/ball.obj")?,
            transforms: Self::create_transforms(),
        })
    }

    pub fn update(&mut self, dt: f32, ctx: &mut Context) {
        let total_elapsed = self.start_time.elapsed().as_secs_f32();
        for obj in &mut self.transforms {
            obj.position.y =
                (total_elapsed - obj.position.x * 0.2 + obj.position.z * 0.3).sin() / 2.;
            obj.rotation.rotate_y(obj.position.x * 0.03 * dt);
        }

        ctx.renderer.update_instances(&[
            (self.cube_model, &self.transforms[..8]),
            (self.ball_model, &self.transforms[8..]),
        ]);
    }

    fn create_transforms() -> Vec<Transform> {
        const SPACE_BETWEEN: f32 = 3.0;
        const INSTANCES_PER_ROW: u32 = 4;

        (0..INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..INSTANCES_PER_ROW).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32 - INSTANCES_PER_ROW as f32);
                    let z = SPACE_BETWEEN * (z as f32 - INSTANCES_PER_ROW as f32);

                    let position = Vec3 { x, y: 0.0, z };

                    let rotation = if position == Vec3::zero() {
                        Quaternion::identity()
                    } else {
                        Quaternion::rotation_3d(1., position)
                    };

                    Transform {
                        position,
                        rotation,
                        scale: Vec3::broadcast(1.0),
                    }
                })
            })
            .collect::<Vec<_>>()
    }
}
