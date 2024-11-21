use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use std::sync::{Arc, Mutex};
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

struct App<'a> {
    window: Option<Arc<Window>>,
    state: Option<State<'a>>,
    // master: Master,
    // oscillators: Arc<Mutex<Vec<Oscillator>>>,
}

impl App<'_> {
    pub fn new(oscillators: Arc<Mutex<Vec<Oscillator>>>) -> Self {
        Self {
            window: None,
            state: None,
            // master: Master::new(oscillators.clone()),
            // oscillators,
        }
    }
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(Window::default_attributes())
                    .unwrap(),
            );
            self.window = Some(window.clone());
            let state = pollster::block_on(State::new(window.clone()));
            self.state = Some(state);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(state) = &mut self.state {
                    state.resize(physical_size);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(state) = &mut self.state {
                    state.render().unwrap();
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key,
                        state,
                        ..
                    },
                ..
            } => match physical_key {
                PhysicalKey::Code(KeyCode::Escape) => {
                    event_loop.exit();
                }
                PhysicalKey::Code(KeyCode::KeyA) => {
                    // We want to block current thread here so we dont loose an input
                    // if let Ok(mut lock) = self.oscillators.lock() {
                    //     for osc in lock.iter_mut() {
                    //         osc.active = state.is_pressed();
                    //         osc.freq = 440.0;
                    //     }
                    // }
                }
                _ => {}
            },
            _ => (),
        }
    }
}

pub struct State<'a> {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl<'a> State<'a> {
    pub async fn new(window: Arc<Window>) -> State<'a> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web, we'll have to disable some.
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                    memory_hints: Default::default(),
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        Self {
            instance,
            surface,
            device,
            queue,
            config,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn main() {
    simple_logger::init().unwrap();
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let oscillators = Arc::new(Mutex::new(vec![
        Oscillator::new(Waveform::Sin, 440.0),
        // Oscillator::new(Waveform::Triangle, 340.0),
    ]));

    let mut app = App::new(oscillators);
    let _ = event_loop.run_app(&mut app);
}

pub struct Master {
    host: cpal::Host,
    device: cpal::Device,
    config: cpal::StreamConfig,
    stream: cpal::Stream,
}

impl Master {
    pub fn new(oscillators: Arc<Mutex<Vec<Oscillator>>>) -> Master {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");

        let config = Self::config(&device);
        let channels = config.channels.into();
        let sample_rate = config.sample_rate.0 as f32;

        let volume = 0.5;

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    for frame in data.chunks_mut(channels) {
                        let mut accum = 0.0;

                        if let Ok(mut lock) = oscillators.try_lock() {
                            for osc in lock.iter_mut() {
                                let value = osc.sample(sample_rate);
                                accum += value;
                            }

                            for sample in frame.iter_mut() {
                                *sample = accum / lock.len() as f32 * volume;
                            }
                        } else {
                            println!("audio stream blocked!");
                        }
                    }
                },
                move |err| {
                    eprintln!("Error in stream callback: {err}");
                },
                None, // None=blocking, Some(Duration)=timeout
            )
            .unwrap();

        stream.play().unwrap();

        Master {
            host,
            device,
            config,
            stream,
        }
    }

    fn config(device: &cpal::Device) -> cpal::StreamConfig {
        let supported_configs_range = device
            .supported_output_configs()
            .expect("error while querying configs");
        for config in supported_configs_range {
            if SampleFormat::is_float(&config.sample_format()) {
                if let Some(config) = config.try_with_sample_rate(cpal::SampleRate(44_100)) {
                    println!("Selected config: {config:?}");
                    return config.into();
                }
            }
        }

        panic!("could not find f32 44.1 kH config");
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Waveform {
    Sin,
    Square,
    Triangle,
}

#[derive(Debug)]
pub struct Oscillator {
    waveform: Waveform,
    phase: f32,
    freq: f32,
    active: bool,
}

impl Oscillator {
    pub fn new(waveform: Waveform, freq: f32) -> Self {
        Self {
            waveform,
            freq,
            phase: 0.0,
            active: false,
        }
    }

    pub fn sample(&mut self, sample_rate: f32) -> f32 {
        if !self.active {
            return 0.0;
        }

        let sample = match self.waveform {
            Waveform::Sin => (self.phase * std::f32::consts::TAU).sin(),
            Waveform::Square => {
                let value = (self.phase * std::f32::consts::TAU).sin();
                if value > 0. {
                    1.0
                } else if value < 0. {
                    -1.0
                } else {
                    0.
                }
            }
            Waveform::Triangle => 2.0 * (self.phase - 0.5).abs() * 2.0 - 1.0,
        };

        self.phase += self.freq / sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        sample
    }
}
