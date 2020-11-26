use std::sync::Arc;
use std::time::Instant;

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use vulkano::buffer::BufferAccess;
use vulkano::buffer::BufferUsage;
use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::command_buffer::AutoCommandBuffer;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::DynamicState;
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::format::Format;
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract, Subpass};
use vulkano::image::{AttachmentImage, ImageUsage, SwapchainImage};
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::GraphicsPipelineAbstract;
use vulkano::swapchain;
use vulkano::swapchain::AcquireError;
use vulkano::swapchain::ColorSpace;
use vulkano::swapchain::FullscreenExclusive;
use vulkano::swapchain::PresentMode;
use vulkano::swapchain::Surface;
use vulkano::swapchain::SurfaceTransform;
use vulkano::swapchain::Swapchain;
use vulkano::swapchain::SwapchainCreationError;
use vulkano::sync::{self, FlushError, GpuFuture};

use vulkano_win::VkSurfaceBuild;
use winit::event::Event;
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

#[derive(Default, Debug, Clone)]
struct Vertex {
    position: [f32; 2],
}
vulkano::impl_vertex!(Vertex, position);

#[allow(dead_code)]
pub struct Elysium {
    instance: Arc<Instance>,
    physical_device_index: usize,
    device: Arc<Device>,

    graphics_queue: Arc<Queue>,

    command_buffer: Vec<Arc<AutoCommandBuffer>>,
    vertex_buffer: Arc<dyn BufferAccess + Send + Sync>,

    dimensions: [u32; 2],
    swapchain: Arc<Swapchain<Window>>,
    swapchain_images: Vec<Arc<SwapchainImage<Window>>>,

    renderpass: Arc<dyn RenderPassAbstract + Send + Sync>,
    framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    dynamic_state: DynamicState,

    previous_frame_end: Option<Box<dyn GpuFuture>>,
    recreate_swapchain: bool,

    surface: Arc<Surface<Window>>,
    event_loop: Option<EventLoop<()>>,

    vs: vs::Shader,
    fs: fs::Shader,
}

impl Elysium {
    pub fn run(mut self) {
        match self.event_loop {
            Some(_) => {
                let event_loop = self.event_loop.take().unwrap();
                let start_time = Instant::now();
                let mut last_time = start_time.clone();
                event_loop.run(move |event, _, control_flow| {
                    match event {
                        Event::WindowEvent {
                            event: WindowEvent::CloseRequested,
                            ..
                        } => {
                            *control_flow = ControlFlow::Exit;
                        }
                        Event::WindowEvent {
                            event: WindowEvent::Resized(_),
                            ..
                        } => {
                            self.recreate_swapchain = true;
                        }
                        Event::MainEventsCleared => {
                            //if start_time.elapsed() > Duration::from_secs(1) {
                            //    *control_flow = ControlFlow::Exit;
                            //}
                            self.draw();
                            //println!(
                            //    "elapsed: {:?}\tdelta: {:?}",
                            //    start_time.elapsed(),
                            //    last_time.elapsed()
                            //);
                            print!("\rfps: {:?}", 1f64 / last_time.elapsed().as_secs_f64());
                            last_time = Instant::now();
                        }
                        _ => (),
                    }
                });
            }
            None => self.reinit(),
        }
    }

    fn reinit(&mut self) {
        todo!("Need to implement this maybe");
    }

    fn draw(&mut self) {
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        if self.recreate_swapchain {
            let dims: [u32; 2] = self.surface.window().inner_size().into();

            let (new_swapchain, new_images) = match self.swapchain.recreate_with_dimensions(dims) {
                Ok(r) => r,
                Err(SwapchainCreationError::UnsupportedDimensions) => return,
                Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
            };

            self.swapchain = new_swapchain;
            self.swapchain_images = new_images;

            let (framebuffers, dynamic_state) = Self::create_framebuffers(
                self.device.clone(),
                self.renderpass.clone(),
                &self.swapchain_images,
            );

            self.framebuffers = framebuffers;
            self.dynamic_state = dynamic_state;

            self.dimensions = dims;
            self.recreate_swapchain = false;
        }

        let (image_num, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    return;
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };

        if suboptimal {
            self.recreate_swapchain = true;
        }

        let clear_values = vec![[0.0, 0.0, 1.0, 1.0].into(), 1f32.into()];

        let mut builder = AutoCommandBufferBuilder::primary_one_time_submit(
            self.device.clone(),
            self.graphics_queue.family(),
        )
        .unwrap();

        builder
            .begin_render_pass(self.framebuffers[image_num].clone(), false, clear_values)
            .unwrap()
            .draw(
                self.pipeline.clone(),
                &self.dynamic_state,
                vec![self.vertex_buffer.clone()],
                (),
                (),
            )
            .unwrap()
            .end_render_pass()
            .unwrap();

        let command_buffer = builder.build().unwrap();

        let future = self
            .previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.graphics_queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.graphics_queue.clone(),
                self.swapchain.clone(),
                image_num,
            )
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                self.previous_frame_end = Some(future.boxed());
            }
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
            Err(e) => {
                warn!("Failed to flush future: {:?}", e);
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
        }
    }

    pub fn new() -> Self {
        let instance = Self::create_instance();

        let physical_device_index = Self::create_physical(instance.clone());
        let physical = PhysicalDevice::from_index(&instance, physical_device_index).unwrap();
        info!("Physical device index: {}", physical_device_index);

        let event_loop = EventLoop::new();
        info!("Created new EventLoop");

        let surface = WindowBuilder::new()
            .build_vk_surface(&event_loop, instance.clone())
            .unwrap();

        let dimensions = surface.window().inner_size().into();

        info!("Surface build with dimensions: {:?}", dimensions);

        let (device, graphics_queue) = Self::create_device_and_queue(physical, surface.clone());

        let (swapchain, swapchain_images) = Self::create_swapchain(
            device.clone(),
            physical,
            surface.clone(),
            dimensions,
            graphics_queue.clone(),
        )
        .unwrap();

        let renderpass = Self::create_renderpass(device.clone(), swapchain.clone());

        let (framebuffers, dynamic_state) =
            Self::create_framebuffers(device.clone(), renderpass.clone(), &swapchain_images);

        let previous_frame_end = Some(sync::now(device.clone()).boxed());

        let vertex_buffer = {
            CpuAccessibleBuffer::from_iter(
                device.clone(),
                BufferUsage::all(),
                false,
                [
                    Vertex {
                        position: [-0.5, -0.25],
                    },
                    Vertex {
                        position: [0.0, 0.5],
                    },
                    Vertex {
                        position: [0.25, -0.1],
                    },
                ]
                .iter()
                .cloned(),
            )
            .unwrap()
        };

        let vs = vs::Shader::load(device.clone()).unwrap();
        let fs = fs::Shader::load(device.clone()).unwrap();

        let pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<Vertex>()
                .vertex_shader(vs.main_entry_point(), ())
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .fragment_shader(fs.main_entry_point(), ())
                .render_pass(Subpass::from(renderpass.clone(), 0).unwrap())
                .build(device.clone())
                .unwrap(),
        );

        Self {
            instance,
            physical_device_index,
            device,
            graphics_queue,
            command_buffer: vec![],
            vertex_buffer,
            dimensions,
            swapchain,
            swapchain_images,
            renderpass,
            dynamic_state,
            framebuffers,
            pipeline,
            previous_frame_end,
            recreate_swapchain: false,
            surface,
            event_loop: Some(event_loop),
            vs,
            fs,
        }
    }

    fn create_instance() -> Arc<Instance> {
        let required_extensions = vulkano_win::required_extensions();
        Instance::new(None, &required_extensions, None)
            .expect("Couldn't create the Vulkan instance")
    }

    fn create_physical(instance: Arc<Instance>) -> usize {
        let physical = PhysicalDevice::enumerate(&instance)
            .next()
            .expect("No device chosen");
        info!("Using physical device: {:?}", physical.name());
        physical.index()
    }

    fn create_device_and_queue(
        physical: PhysicalDevice,
        surface: Arc<Surface<Window>>,
    ) -> (Arc<Device>, Arc<Queue>) {
        let device_ext = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::none()
        };

        info!(
            "GPU has {} queue family(ies)",
            physical.queue_families().len()
        );
        let queue_family = physical
            .queue_families()
            .find(|&q| q.supports_graphics() && surface.is_supported(q).unwrap_or(false))
            .unwrap();

        let (device, mut queues) = Device::new(
            physical,
            physical.supported_features(),
            &device_ext,
            [(queue_family, 0.5)].iter().cloned(),
        )
        .unwrap();

        (device, queues.next().unwrap())
    }

    fn create_swapchain(
        device: Arc<Device>,
        physical: PhysicalDevice,
        surface: Arc<Surface<Window>>,
        dimensions: [u32; 2],
        graphics_queue: Arc<Queue>,
    ) -> Result<(Arc<Swapchain<Window>>, Vec<Arc<SwapchainImage<Window>>>), SwapchainCreationError>
    {
        let caps = surface.capabilities(physical).unwrap();
        let format = caps.supported_formats[0].0;
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();

        Swapchain::new(
            device.clone(),
            surface.clone(),
            caps.min_image_count,
            format,
            dimensions,
            1,
            ImageUsage::color_attachment(),
            &graphics_queue,
            SurfaceTransform::Identity,
            alpha,
            PresentMode::Fifo,
            FullscreenExclusive::Default,
            true,
            ColorSpace::SrgbNonLinear,
        )
    }

    fn create_renderpass(
        device: Arc<Device>,
        swapchain: Arc<Swapchain<Window>>,
    ) -> Arc<dyn RenderPassAbstract + Send + Sync> {
        Arc::new(
            vulkano::single_pass_renderpass!(device.clone(),
                attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.format(),
                    samples: 1,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: Format::D16Unorm,
                    samples: 1,
                }
                },
                pass: {
                    color: [color],
                    depth_stencil: {depth}
                }
            )
            .unwrap(),
        )
    }

    fn create_framebuffers(
        device: Arc<Device>,
        renderpass: Arc<dyn RenderPassAbstract + Send + Sync>,
        images: &[Arc<SwapchainImage<Window>>],
    ) -> (
        Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
        DynamicState,
    ) {
        let dimensions = images[0].dimensions();

        let depth_buffer =
            AttachmentImage::transient(device.clone(), dimensions, Format::D16Unorm).unwrap();

        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [dimensions[0] as f32, dimensions[1] as f32],
            depth_range: 0.0..1.0,
        };

        let dynamic_state = DynamicState {
            line_width: None,
            viewports: Some(vec![viewport]),
            scissors: None,
            compare_mask: None,
            write_mask: None,
            reference: None,
        };

        let framebuffers = images
            .iter()
            .map(|image| {
                Arc::new(
                    Framebuffer::start(renderpass.clone())
                        .add(image.clone())
                        .unwrap()
                        .add(depth_buffer.clone())
                        .unwrap()
                        .build()
                        .unwrap(),
                ) as Arc<dyn FramebufferAbstract + Send + Sync>
            })
            .collect::<Vec<_>>();

        (framebuffers, dynamic_state)
    }

    //fn create_command_buffers(&mut self) {
    //    let queue_family = self.graphics_queue.family();
    //    self.command_buffer = self.swapchain_images.iter()
    //        .map(|framebuffer| {
    //            Arc::new(AutoCommandBufferBuilder::primary_simultaneous_use(self.device.clone(), queue_family)
    //                .unwrap()
    //                .begin_render_pass(framebuffer.clone(), false, vec![[0.0, 0.0, 1.0, 1.0].into()])
    //                .unwrap()
    //                .draw_indexed(pipeline, dynamic, vertex_buffer, index_buffer, sets, constants)
    //        })
    //}
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
                            #version 450
                            layout(location = 0) in vec2 position;
                            void main() {
                                    gl_Position = vec4(position, 0.0, 1.0);
                            }
                    "
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
                            #version 450
                            layout(location = 0) out vec4 f_color;
                            void main() {
                                    f_color = vec4(1.0, 0.0, 0.0, 1.0);
                            }
                    "
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert!(true);
    }
}
