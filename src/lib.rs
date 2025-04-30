use wgpu::util::DeviceExt;

const WORKGROUP_SIZE: u32 = 8;

pub struct LifeSimulation {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub pipeline_layout: wgpu::PipelineLayout,
    pub compute_pipeline: wgpu::ComputePipeline,
    pub bind_groups: [wgpu::BindGroup; 2],
    pub step: u64,

    pub grid_size: u32,
    pub num_cells: usize,
}

impl LifeSimulation {
    pub async fn new(grid_size: u32) -> Self {
        // Make sure the grid size is a multiple of the workgroup size so that
        // the simulation can be broken up cleanly into workgroups.
        assert!(
            grid_size as u32 % WORKGROUP_SIZE == 0,
            "Grid size {grid_size} is not divisible by {WORKGROUP_SIZE}",
        );

        let num_cells = (grid_size * grid_size) as usize;

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
            contents: bytemuck::cast_slice(&[grid_size as f32, grid_size as f32]),
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

        let mut initial_cell_states = vec![0u32; num_cells as usize];
        for i in 0..num_cells {
            initial_cell_states[i] = rand::random::<u32>() % 2;
        }

        let cell_state_buffer_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cell State Buffer"),
            contents: bytemuck::cast_slice(&initial_cell_states),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        for i in 0..num_cells {
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
            grid_size,
            num_cells,
        }
    }

    pub fn encode_compute_pass(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.bind_groups[(self.step % 2) as usize], &[]);
        compute_pass.dispatch_workgroups(
            self.grid_size / WORKGROUP_SIZE,
            self.grid_size / WORKGROUP_SIZE,
            1,
        );

        drop(compute_pass);

        self.step += 1;
    }
}
