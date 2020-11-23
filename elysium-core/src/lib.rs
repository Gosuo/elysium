#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_import_braces)]
use std::sync::Arc;
use std::thread;

use log::{debug, error, info, warn};

use vulkano::command_buffer::pool::CommandPool;
use vulkano::command_buffer::pool::CommandPoolBuilderAlloc;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::swapchain::Surface;
use vulkano::{
    command_buffer::{pool::standard::StandardCommandPoolBuilder, AutoCommandBuffer},
    image::ImageUsage,
    image::SwapchainImage,
    swapchain::FullscreenExclusive,
    swapchain::{ColorSpace, PresentMode, SurfaceTransform, Swapchain},
};

use vulkano_win::VkSurfaceBuild;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

#[allow(dead_code)]
pub struct Elysium {
    vk_instance: Arc<Instance>,
    vk_physical_device_index: usize,
    vk_device: Arc<Device>,

    graphics_queue: Arc<Queue>,

    command_buffer_pool: Arc<AutoCommandBufferBuilder>,

    dimensions: [u32; 2],
    swapchain: Arc<Swapchain<Window>>,
    swapchain_images: Vec<Arc<SwapchainImage<Window>>>,

    surface: Arc<Surface<Window>>,
    event_loop: EventLoop<()>,
}

impl Elysium {
    pub fn run(&self) -> thread::JoinHandle<()> {
        thread::spawn(|| println!("Hello from render thread"))
    }

    pub fn new() -> Self {
        let vk_instance = {
            let required_extensions = vulkano_win::required_extensions();
            Instance::new(None, &required_extensions, None)
                .expect("Couldn't create the Vulkan instance")
        };

        let vk_physical_device_index = {
            let physical = PhysicalDevice::enumerate(&vk_instance)
                .next()
                .expect("No device chosen");
            info!("Using physical device: {:?}", physical.name());
            physical.index()
        };

        info!("Physical device index: {}", vk_physical_device_index);

        let event_loop = EventLoop::new();
        info!("Created new EventLoop");

        let surface = WindowBuilder::new()
            .build_vk_surface(&event_loop, vk_instance.clone())
            .unwrap();

        let dimensions = surface.window().inner_size().into();

        info!("Surface build with dimensions: {:?}", dimensions);

        let physical = PhysicalDevice::from_index(&vk_instance, vk_physical_device_index).unwrap();

        let (device, mut queues) = {
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

            Device::new(
                physical,
                physical.supported_features(),
                &device_ext,
                [(queue_family, 0.5)].iter().cloned(),
            )
            .unwrap()
        };

        info!("{} graphic queue(s) available", queues.len());

        let graphics_queue = queues.next().unwrap();

        let (mut swapchain, swapchain_images) = {
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
            .unwrap()
        };

        let command_buffer_pool = Arc::new(
            AutoCommandBufferBuilder::new(device.clone(), graphics_queue.family()).unwrap(),
        );

        let vk_device = device;

        Self {
            vk_instance,
            vk_physical_device_index,
            vk_device,
            graphics_queue,
            command_buffer_pool,
            dimensions,
            swapchain,
            swapchain_images,
            surface,
            event_loop,
        }
    }

    //pub fn init(&mut self) {
    //    self.event_loop = EventLoop::new();
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
