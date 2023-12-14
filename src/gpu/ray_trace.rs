use super::type_mapping;
use crate::color::Color;
use crate::world::World;
use glam::f32::*;
use metal::*;
use objc::rc::autoreleasepool;

pub(crate) fn render(world: &World) -> Vec<Color> {
    let width = world.camera.image_width;
    let height = world.camera.image_height;

    autoreleasepool(|| {
        let device = Device::system_default().expect("No device found");

        let counter_sampling_point = MTLCounterSamplingPoint::AtStageBoundary;
        assert!(device.supports_counter_sampling(counter_sampling_point));

        let command_queue = device.new_command_queue();
        let command_buffer = command_queue.new_command_buffer();

        let compute_pass_descriptor = ComputePassDescriptor::new();
        let encoder =
            command_buffer.compute_command_encoder_with_descriptor(compute_pass_descriptor);

        let pipeline_state = create_pipeline_state(&device);
        encoder.set_compute_pipeline_state(&pipeline_state);

        let buffers = create_buffers(&device, &world);

        encoder.set_buffer(0, Some(&buffers.output), 0);
        encoder.set_buffer(1, Some(&buffers.uniforms), 0);
        encoder.set_buffer(2, Some(&buffers.camera), 0);
        encoder.set_buffer(3, Some(&buffers.spheres), 0);

        let num_threads = pipeline_state.thread_execution_width();

        let threads_per_thread_group = MTLSize::new(num_threads, num_threads, 1);
        let thread_groups = MTLSize::new(
            (width as u64 + threads_per_thread_group.width - 1) / threads_per_thread_group.width,
            (height as u64 + threads_per_thread_group.height - 1) / threads_per_thread_group.height,
            1,
        );

        encoder.dispatch_thread_groups(thread_groups, threads_per_thread_group);
        encoder.end_encoding();

        command_buffer.commit();
        command_buffer.wait_until_completed();

        let ptr = buffers.output.contents() as *mut Vec3A;
        let mut data = vec![];

        unsafe {
            for i in 0..width * height {
                let v = *ptr.add(i as usize);
                data.push(Color::new(v.x, v.y, v.z));
            }
        };

        return data;
    })
}

const SHADER_FILE: &str = super::shader::shader_file();

fn create_pipeline_state(device: &Device) -> ComputePipelineState {
    let library = device
        .new_library_with_source(SHADER_FILE, &CompileOptions::new())
        .unwrap_or_else(|err| {
            println!("Failed to create library: {}", err);
            std::process::exit(1);
        });
    let kernel = library.get_function("ray_trace", None).unwrap();

    let pipeline_state_descriptor = ComputePipelineDescriptor::new();
    pipeline_state_descriptor.set_compute_function(Some(&kernel));

    device
        .new_compute_pipeline_state_with_function(
            pipeline_state_descriptor.compute_function().unwrap(),
        )
        .unwrap()
}

struct Buffers {
    camera: metal::Buffer,
    output: metal::Buffer,
    uniforms: metal::Buffer,
    spheres: metal::Buffer,
}

fn create_buffers(device: &Device, data: &World) -> Buffers {
    let width = data.camera.image_width;
    let height = data.camera.image_height;
    let spheres: &Vec<type_mapping::Sphere> = &data.spheres;
    let camera: &type_mapping::Camera = &data.camera;

    println!("{:?}", camera);
    println!("{:?}", spheres);

    let camera = device.new_buffer_with_data(
        unsafe { std::mem::transmute(&camera) },
        std::mem::size_of::<type_mapping::Camera>() as u64,
        MTLResourceOptions::CPUCacheModeDefaultCache,
    );

    let uniforms = {
        let uniforms = type_mapping::Uniforms {
            samples: 1,
            sphere_count: spheres.len() as u32,
        };
        device.new_buffer_with_data(
            unsafe { std::mem::transmute(&uniforms) },
            std::mem::size_of::<type_mapping::Uniforms>() as u64,
            MTLResourceOptions::CPUCacheModeDefaultCache,
        )
    };

    let spheres = {
        device.new_buffer_with_data(
            unsafe { std::mem::transmute(spheres.as_ptr()) },
            (spheres.len() * std::mem::size_of::<type_mapping::Camera>()) as u64,
            MTLResourceOptions::CPUCacheModeDefaultCache,
        )
    };

    let output = {
        let data = vec![Vec3A::new(0.0, 0.0, 0.0); (width * height) as usize];
        device.new_buffer_with_data(
            unsafe { std::mem::transmute(data.as_ptr()) },
            (data.len() * std::mem::size_of::<Vec3A>()) as u64,
            MTLResourceOptions::CPUCacheModeDefaultCache,
        )
    };

    Buffers {
        camera,
        uniforms,
        spheres,
        output,
    }
}
