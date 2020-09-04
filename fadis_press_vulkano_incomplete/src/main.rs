use vulkano::{
    app_info_from_cargo_toml,
    device::{Device, DeviceExtensions, Features},
    instance::{Instance, PhysicalDevice},
};
use vulkano_win::VkSurfaceBuild;
use winit::{dpi::LogicalSize, event_loop::EventLoop, window::WindowBuilder};

const WIDTH: u32 = 600;
const HEIGHT: u32 = 800;

fn main() {
    // インスタンスの作成
    let app_info = app_info_from_cargo_toml!();

    let mut ext = vulkano_win::required_extensions();
    ext.ext_debug_utils = true;

    let layers = ["VK_LAYER_LUNARG_standard_validation"];

    let instance = match Instance::new(Some(&app_info), &ext, layers.iter().cloned()) {
        Ok(i) => i,
        Err(err) => panic!("failed to create Vulkan instance: {:?}", err),
    };

    // 物理デバイスの表示
    for device in PhysicalDevice::enumerate(&instance) {
        println!("Name: {}", device.name());
        println!("Index: {}", device.index());
        println!("Type: {:?}", device.ty());
        println!("API version: {}", device.api_version());
    }

    // サーフェスを作成する
    let event_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .with_title("Vulkan")
        .with_inner_size(LogicalSize::new(f64::from(WIDTH), f64::from(HEIGHT)))
        .build_vk_surface(&event_loop, instance.clone())
        .expect("failed to create window surface");

    // デバイスに要求する拡張
    // ここではswapchain拡張
    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };

    // サーフェスに適合する物理デバイスを探す
    let physical_device = PhysicalDevice::enumerate(&instance)
        .find(|d| {
            // 必要な拡張を持っているか
            let available_extensions = DeviceExtensions::supported_by_device(*d);
            let is_swapchain_supported =
                available_extensions.intersection(&device_extensions) == device_extensions;

            // Surfaceの能力の確認
            // サポートしているフォーマットが一つ以上あり、
            // present modesが一つ以上存在するか調べている
            let capabilities = surface
                .capabilities(*d)
                .expect("failed to get surface capabilities");
            let is_swapchain_adequate = !capabilities.supported_formats.is_empty()
                && capabilities.present_modes.iter().next().is_some();

            // deviceのqueue_familyに必要な機能が揃っているか確認
            // graphicsをサポートするfamilyと、
            // Surfaceにpresentできるfamilyが存在することを確認している
            // 一つのfamilyで両方に対応していることもある
            let mut supports_graphics = false;
            let mut present_family = false;
            for queue_family in d.queue_families() {
                if queue_family.supports_graphics() {
                    supports_graphics = true;
                }
                if surface.is_supported(queue_family).unwrap() {
                    present_family = true;
                }
                if supports_graphics && present_family {
                    break;
                }
            }

            is_swapchain_supported && is_swapchain_adequate && supports_graphics && present_family
        })
        .expect("failed to get suitable device");

    // 論理デバイスの作成
    // キューも同時に返す
    let (_device, _graphics_queue, _present_queue) = {
        // キューファミリーを取得
        // graphics用とpresent用の2つ
        // ただし、この2つは同一のこともある
        // 実用上は並列化のために別のキューのほうが望ましい？
        let mut graphics_index = -1;
        let mut present_index = -1;
        for (i, queue_family) in physical_device.queue_families().enumerate() {
            if queue_family.supports_graphics() {
                graphics_index = i as i32;
            }
            if surface.is_supported(queue_family).unwrap() {
                present_index = i as i32;
            }
            if graphics_index >= 0 && present_index >= 0 {
                break;
            }
        }
        // 論理デバイス作成のために必要なqueueとpriorityのタプルのイテレータを作成
        use std::collections::HashSet;
        let queue_priority = 1.0;
        let queue_families_indices: HashSet<usize> = [graphics_index, present_index]
            .iter()
            .map(|i| *i as usize)
            .collect();
        let queue_families = queue_families_indices.into_iter().map(|i| {
            (
                physical_device.queue_families().nth(i).unwrap(),
                queue_priority,
            )
        });
        // deviceだけでなくキューも返す
        let (device, mut queues) = Device::new(
            physical_device,
            &Features::none(),
            &device_extensions,
            queue_families,
        )
        .expect("failed to create logical device");
        let graphics_queue = queues.next().unwrap();
        let present_queue = queues.next().unwrap_or_else(|| graphics_queue.clone());
        (device, graphics_queue, present_queue)
    };
}
