#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_import_braces)]
use std::sync::Arc;
use std::thread;

use vulkano::command_buffer::pool::standard::StandardCommandPoolBuilder;
use vulkano::command_buffer::pool::CommandPool;
use vulkano::command_buffer::pool::CommandPoolBuilderAlloc;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::swapchain::Surface;

use vulkano_win::VkSurfaceBuild;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

pub struct Elysium {
    vk_instance: Arc<Instance>,
    vk_physical_device_index: usize,
    vk_device: Arc<Device>,

    graphics_queue: Arc<Queue>,

    dimensions: [u32; 2],

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
            physical.index()
        };

        let event_loop = EventLoop::new();

        let surface = WindowBuilder::new()
            .build_vk_surface(&event_loop, vk_instance.clone())
            .unwrap();

        let dimensions = surface.window().inner_size().into();

        let (device, mut queues) = {
            let device_ext = DeviceExtensions {
                khr_swapchain: true,
                ..DeviceExtensions::none()
            };

            let physical =
                PhysicalDevice::from_index(&vk_instance, vk_physical_device_index).unwrap();

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

        let vk_device = device;
        let graphics_queue = queues.next().unwrap();

        Self {
            vk_instance,
            vk_physical_device_index,
            vk_device,
            graphics_queue,
            dimensions,
            surface,
            event_loop,
        }
    }

    fn init_vulkan(&mut self) {
        self.vk_instance = {
            let required_extensions = vulkano_win::required_extensions();
            Instance::new(None, &required_extensions, None)
                .expect("Couldn't create the Vulkan instance")
        };

        self.vk_physical_device_index = {
            let physical = PhysicalDevice::enumerate(&self.vk_instance)
                .next()
                .expect("No device chosen");
            physical.index()
        };

        self.surface = WindowBuilder::new()
            .build_vk_surface(&self.event_loop, self.vk_instance.clone())
            .unwrap();

        self.dimensions = self.surface.window().inner_size().into();

        let (device, mut queues) = {
            let device_ext = DeviceExtensions {
                khr_swapchain: true,
                ..DeviceExtensions::none()
            };

            let physical =
                PhysicalDevice::from_index(&self.vk_instance, self.vk_physical_device_index)
                    .unwrap();

            let queue_family = physical
                .queue_families()
                .find(|&q| q.supports_graphics() && self.surface.is_supported(q).unwrap_or(false))
                .unwrap();

            Device::new(
                physical,
                physical.supported_features(),
                &device_ext,
                [(queue_family, 0.5)].iter().cloned(),
            )
            .unwrap()
        };

        self.vk_device = device;
        self.graphics_queue = queues.next().unwrap();
    }

    pub fn init(&mut self) {
        self.event_loop = EventLoop::new();
        self.init_vulkan();
    }
}

#[cfg(test)]
mod tests {
    use super::Elysium;

    #[test]
    fn it_works() {
        assert!(true);
    }
}
