use std::{ops::ControlFlow, time::Instant};

use anyhow::Context as _;

use egui::Context as EguiContext;
use egui_winit::State as EguiWinitState;

use common::{Quaternion, Transform, Vec2, Vec3};
use game_engine::{
    rendering::{model::ModelIndex, Line, Renderer},
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
    state: EguiWinitState,
    egui_context: EguiContext,
    context: Context,
    camera_controller: CameraController,
    scene: ExampleScene,
    last_frame: Instant,
    render_texture: egui::TextureHandle,
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

        let icon = image::open("res/icon.png")
            .with_context(|| format!("failed to open icon.png"))?
            .into_rgba8();
        let (icon_width, icon_height) = icon.dimensions();
        let icon = winit::window::Icon::from_rgba(icon.into_raw(), icon_width, icon_height)
            .with_context(|| format!("failed to convert the icon image into an icon"))?;

        let window = Window::new(&event_loop, Some(icon))
            .with_context(|| format!("failed to open the winit window"))?;
        let mut context = Context {
            renderer: Renderer::new(
                window.raw_window_handle(),
                window.inner_size(),
                [0.229, 0.507, 0.921],
            )
            .with_context(|| format!("failed to create the renderer"))?,
        };

        let camera_controller = CameraController::new(
            10.0,
            0.1,
            Vec3::new(-16.0, 4.0, 1.0),
            Vec2::new(-0.3, 135f32.to_radians()),
        );

        let scene =
            ExampleScene::new(&mut context).with_context(|| "failed to create the scene")?;
        let state = EguiWinitState::new(4096, &window.winit_window());
        let egui_context = EguiContext::default();
        {
            let mut opts = egui_context.tessellation_options();
            //opts.debug_paint_clip_rects = true;
            opts.anti_alias = false;
            //opts.debug_paint_text_rects = true;
        }
        let render_texture = context.renderer.make_egui_render_target(&egui_context);

        Ok((
            event_loop,
            Self {
                window,
                state,
                egui_context,
                context,
                camera_controller,
                scene,
                last_frame: Instant::now(),
                render_texture,
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
                log::info!("Resized window, new size: ({}, {})", width, height);
                self.context.renderer.resize((width, height));
                self.window.update_size();
            }
            Event::WindowEvent { event, .. } => {
                self.camera_controller.process_window_events(&event);
                self.state.on_event(&self.egui_context, &event);
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

        let mut lines = Vec::new();
        let start = self.scene.transforms[0].position;
        let color = Vec3::new(1.0, 0.0, 0.0);

        for &Transform { position: end, .. } in &self.scene.transforms[1..] {
            lines.push(Line { start, end, color });
        }

        let raw_input = self.state.take_egui_input(&self.window.winit_window());
        let full_output = self.egui_context.run(raw_input, |ctx| {
            egui::Window::new("my_area").auto_sized().show(&ctx, |ui| {
                ui.label("Hello world!");
                //ui.label("More texttttt");
                ui.image(&self.render_texture, (400.0, 300.0));
            });
        });

        if self.window.inner_size() != (0, 0) {
            match self.context.renderer.render(
                &lines,
                &self.egui_context,
                full_output,
                self.egui_context.pixels_per_point(),
            ) {
                Ok(_) => (),
                Err(e) => log::error!("Failed to render: {}", e),
            };
        }
    }
}

impl ExampleScene {
    pub fn new(context: &mut Context) -> anyhow::Result<Self> {
        Ok(Self {
            start_time: Instant::now(),
            cube_model: context
                .renderer
                .load_model("res/cube.obj")
                .with_context(|| "failed to open cube.obj")?,
            ball_model: context
                .renderer
                .load_model("res/ball.obj")
                .with_context(|| "failed to open ball.obj")?,
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

        {
            let mut mgr = ctx.renderer.get_models_mut();
            mgr.set_transforms(self.cube_model, self.transforms[..8].to_vec());
            mgr.set_transforms(self.ball_model, self.transforms[8..].to_vec());
        }
        // ctx.renderer.update_instances(&[
        //     (self.cube_model, &self.transforms[..8]),
        //     (self.ball_model, &self.transforms[8..]),
        // ]);
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
