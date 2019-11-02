use std::borrow::Cow;
use vulkano::device::{Device, DeviceExtensions, Features};
use vulkano::instance::{ApplicationInfo, Instance, InstanceExtensions, PhysicalDevice, Version};

fn main() {
    let app_info = ApplicationInfo {
        application_name: Some(Cow::from("application")),
        application_version: Some(Version {
            major: 1,
            minor: 0,
            patch: 0,
        }),
        engine_name: Some(Cow::from("renderer")),
        engine_version: Some(Version {
            major: 1,
            minor: 0,
            patch: 0,
        }),
    };
    let extensions = InstanceExtensions::none();
    let instance = Instance::new(Some(&app_info), &extensions, None)
        .unwrap_or_else(|e| panic!("cannot create vulkan instance: {:?}", e));
    let physical = PhysicalDevice::enumerate(&instance)
        .inspect(|device| {
            println!("physical device: {}", device.name());
            println!(" driver version: {}", device.driver_version());
            println!(" api version: {:?}", device.api_version());
        })
        .next()
        .expect("no device available");
    println!("using physical device: {}", physical.name());
    println!("supported features: {:?}", physical.supported_features());

    let graphical_queue = physical
        .queue_families()
        .inspect(|family| {
            println!(
                " family queues: {}, graphics: {}, compute: {}",
                family.queues_count(),
                family.supports_graphics(),
                family.supports_compute()
            );
        })
        .find(|&q| q.supports_graphics())
        .expect("couldn't find a graphical queue family");

    let (device, mut queues) = Device::new(
        physical,
        &Features::none(),
        &DeviceExtensions::none(),
        [(graphical_queue, 0.5)].iter().cloned(),
    )
    .expect("cannot create virtual device");

    // extract the one queue we asked for
    let queue = queues.next().unwrap();
}
