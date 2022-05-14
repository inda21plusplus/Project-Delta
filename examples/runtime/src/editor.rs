use std::{ops::ControlFlow, time::Instant};

use anyhow::Context as _;
use egui::{Context as EguiContext, Slider};
use egui_winit::State as EguiWinitState;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
};

use common::{Vec2, Vec3};
use game_engine::{
    physics,
    rendering::{Line, Renderer},
    Context,
};

use crate::{
    camera_controller::CameraController,
    physics_context::PhysicsScene,
    window::{Window, WindowMode},
};

pub struct Editor {
    window: Window,
    state: EguiWinitState,
    egui_context: EguiContext,
    context: Context,
    camera_controller: CameraController,
    scene: PhysicsScene,
    last_frame: Instant,
}

impl Editor {
    pub fn new() -> anyhow::Result<(EventLoop<()>, Self)> {
        let event_loop = EventLoop::new();

        let icon = image::open("res/icon.png")
            .map(|i| i.into_rgba8())
            .ok()
            .and_then(|icon| {
                let (width, height) = icon.dimensions();
                winit::window::Icon::from_rgba(icon.into_raw(), width, height).ok()
            });
        if icon.is_none() {
            log::warn!("Could not load icon");
        }

        let window = Window::new(&event_loop, icon)
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
            PhysicsScene::new(&mut context).with_context(|| "failed to create the scene")?;
        let state = EguiWinitState::new(4096, &window.winit_window());
        let egui_context = EguiContext::default();
        {
            let mut opts = egui_context.tessellation_options();
            opts.debug_paint_clip_rects = false;
        }

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
                    return self.handle_keyboard_input(keycode, state);
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

    fn handle_keyboard_input(
        &mut self,
        keycode: VirtualKeyCode,
        state: ElementState,
    ) -> ControlFlow<()> {
        if state != ElementState::Pressed {
            return ControlFlow::Continue(());
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
            VirtualKeyCode::Escape => {
                return ControlFlow::Break(());
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn update(&mut self, dt: f32) {
        self.scene.update(dt, &mut self.context);
        self.camera_controller
            .update_camera(dt, &mut self.context.renderer.camera);
        self.context.renderer.update_camera();

        let pos_r = || -100.0..=100.0;
        let k_r = || 0.0..=1.0;

        let raw_input = self.state.take_egui_input(&self.window.winit_window());
        let full_output = self.egui_context.run(raw_input, |ctx| {
            egui::Window::new("The Stuff®")
                .auto_sized()
                .show(&ctx, |ui| {
                    ui.label("Light position");
                    ui.spacing_mut().slider_width *= 2.0;
                    ui.add(Slider::new(&mut self.scene.light.pos.x, pos_r()).text("x"));
                    ui.add(Slider::new(&mut self.scene.light.pos.y, pos_r()).text("y"));
                    ui.add(Slider::new(&mut self.scene.light.pos.z, pos_r()).text("z"));

                    ui.label("scene.Light attenuation factors");
                    ui.add(Slider::new(&mut self.scene.light.k_constant, k_r()).text("k_c"));
                    ui.add(Slider::new(&mut self.scene.light.k_linear, k_r()).text("k_l"));
                    ui.add(Slider::new(&mut self.scene.light.k_quadratic, k_r()).text("k_q"));

                    ui.label("Light color");
                    egui::widgets::color_picker::color_edit_button_rgb(
                        ui,
                        &mut self.scene.light.color,
                    );

                    if let Some(gravity) = self.scene.world.resource_mut::<physics::Gravity>() {
                        ui.label("Gravity");
                        ui.add(Slider::new(&mut gravity.0.y, -20.0..=20.0).text("k_q"));
                    }
                });
        });

        let lines = self
            .scene
            .world
            .resource::<Vec<Line>>()
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        if self.window.inner_size() != (0, 0) {
            match self.context.renderer.render(
                lines,
                &[self.scene.light],
                &self.egui_context,
                full_output,
                self.egui_context.pixels_per_point(),
                false,
            ) {
                Ok(_) => (),
                Err(e) => log::error!("Failed to render: {}", e),
            };
        }
    }
}
