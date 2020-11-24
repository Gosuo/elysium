#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_import_braces)]
use std::sync::Arc;
use std::thread;

use log::{debug, error, info, warn};

use vulkano::{
    command_buffer::{
        pool::{standard::StandardCommandPoolBuilder, CommandPool, CommandPoolBuilderAlloc},
        AutoCommandBuffer, AutoCommandBufferBuilder,
    },
    device::{Device, DeviceExtensions, Queue},
    format::Format,
    framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract},
    image::{ImageUsage, SwapchainImage, AttachmentImage},
    instance::{Instance, PhysicalDevice},
    swapchain::{
        ColorSpace, FullscreenExclusive, PresentMode, Surface, SurfaceCreationError,
        SurfaceTransform, Swapchain, SwapchainCreationError,
    },
};

use vulkano_win::VkSurfaceBuild;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

#[allow(dead_code)]
pub struct Elysium {
    instance: Arc<Instance>,
    physical_device_index: usize,
    device: Arc<Device>,

    graphics_queue: Arc<Queue>,

    command_buffer: Vec<Arc<AutoCommandBuffer>>,

    dimensions: [u32; 2],
    swapchain: Arc<Swapchain<Window>>,
    swapchain_images: Vec<Arc<SwapchainImage<Window>>>,

    renderpass: Arc<dyn RenderPassAbstract + Send + Sync>,
    framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,

    surface: Arc<Surface<Window>>,
    event_loop: EventLoop<()>,
}

impl Elysium {
    pub fn run(&self) -> thread::JoinHandle<()> {
        thread::spawn(|| println!("Hello from render thread"))
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

        let framebuffers = Self::create_framebuffers(device.clone(), renderpass.clone(), &swapchain_images);

        Self {
            instance,
            physical_device_index,
            device,
            graphics_queue,
            command_buffer: vec![],
            dimensions,
            swapchain,
            swapchain_images,
            renderpass,
            framebuffers,
            surface,
            event_loop,
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
    ) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
        let dimensions = images[0].dimensions();

        let depth_buffer =
            AttachmentImage::transient(device.clone(), dimensions, Format::D16Unorm).unwrap();

        images
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
            .collect::<Vec<_>>()
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

#[cfg(test)]
mod tests {
    use super::Elysium;

    #[test]
    fn it_works() {
        assert!(true);
    }
}
