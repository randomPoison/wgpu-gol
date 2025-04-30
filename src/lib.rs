use wgpu::util::DeviceExt;

pub const GRID_SIZE: usize = 256;
const WORKGROUP_SIZE: u32 = 8;

// Make sure the grid is evenly divisible into work groups.
const _: () = assert!(GRID_SIZE as u32 % WORKGROUP_SIZE == 0);

pub struct LifeSimulation {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub pipeline_layout: wgpu::PipelineLayout,
    pub compute_pipeline: wgpu::ComputePipeline,
    pub bind_groups: [wgpu::BindGroup; 2],
    pub step: u64,
}

impl LifeSimulation {
    pub async fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .unwrap();

        dbg!(adapter.get_info());

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let grid_size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Size Buffer"),
            contents: bytemuck::cast_slice(&[GRID_SIZE as f32, GRID_SIZE as f32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Grid Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let mut initial_cell_states = [0u32; GRID_SIZE * GRID_SIZE];
        for i in 0..GRID_SIZE * GRID_SIZE {
            initial_cell_states[i] = rand::random::<u32>() % 2;
        }

        let cell_state_buffer_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cell State Buffer"),
            contents: bytemuck::cast_slice(&initial_cell_states),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        for i in 0..GRID_SIZE * GRID_SIZE {
            initial_cell_states[i] = 0;
        }

        let cell_state_buffer_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cell State Buffer"),
            contents: bytemuck::cast_slice(&initial_cell_states),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grid Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: grid_size_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: cell_state_buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: cell_state_buffer_b.as_entire_binding(),
                },
            ],
        });

        let bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grid Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: grid_size_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: cell_state_buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: cell_state_buffer_a.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let simulation_shader = device.create_shader_module(wgpu::include_wgsl!("simulation.wgsl"));
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Simulation Pipeline"),
            layout: Some(&pipeline_layout),
            module: &simulation_shader,
            entry_point: Some("compute_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self {
            instance,
            adapter,
            device,
            queue,
            pipeline_layout,
            compute_pipeline,
            bind_groups: [bind_group_a, bind_group_b],
            step: 0,
        }
    }

    pub fn encode_compute_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.bind_groups[(self.step % 2) as usize], &[]);
        compute_pass.dispatch_workgroups(
            GRID_SIZE as u32 / WORKGROUP_SIZE,
            GRID_SIZE as u32 / WORKGROUP_SIZE,
            1,
        );

        drop(compute_pass);
    }
}
