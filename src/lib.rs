use std::sync::{atomic::{AtomicBool, Ordering}, Arc};

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
    pub state_bufs: [wgpu::Buffer; 2],
    pub read_buf: wgpu::Buffer,

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

        // Randomly initialize the first state buffer.
        let mut scratch_state = vec![0u32; num_cells as usize];
        for i in 0..num_cells {
            // scratch_state[i] = rand::random::<u32>() % 2;
            scratch_state[i] = 1;
        }

        let cell_state_buffer_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cell State Buffer A"),
            contents: bytemuck::cast_slice(&scratch_state),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        // Zero-initialize the second state buffer.
        for i in 0..num_cells {
            scratch_state[i] = 0;
        }

        let cell_state_buffer_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cell State Buffer B"),
            contents: bytemuck::cast_slice(&scratch_state),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
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

        let read_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Read Buffer"),
            contents: bytemuck::cast_slice(&scratch_state),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
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
            state_bufs: [cell_state_buffer_a, cell_state_buffer_b],
            read_buf,
            step: 0,
            grid_size,
            num_cells,
        }
    }

    // Restarts the simulation
    pub fn reset_state(&mut self, state: &[u32]) {
        assert_eq!(
            state.len(),
            self.num_cells,
            "State data has wrong length, expected {} but got {}",
            self.num_cells,
            state.len(),
        );

        // Reset the step counter so that we're always writing to the first
        // buffer and that buffer will be the input for the next tick.
        self.step = 0;

        self.queue
            .write_buffer(&self.state_bufs[0], 0, bytemuck::cast_slice(state));
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

    /// Tells the GPU to copy the current state of the simulation to the read
    /// buffer.
    ///
    /// This must be called before reading from the read buffer. Trying to read
    /// the state without calling this will yield old state data.
    pub fn encode_read(&self, encoder: &mut wgpu::CommandEncoder) {
        let src_buffer = &self.state_bufs[(self.step % 2) as usize];
        encoder.copy_buffer_to_buffer(src_buffer, 0, &self.read_buf, 0, (self.num_cells * size_of::<u32>()) as u64);
    }

    /// Reads the current grid state from the GPU, blocking until the read
    /// completes.
    pub fn read_state(&self) -> Vec<u32> {
        // Have the GPU copy the current state to the read buffer.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        self.encode_read(&mut encoder);
        self.queue.submit([encoder.finish()]);

        // Wait until the copy operation finishes.
        self.device
            .poll(wgpu::PollType::Wait)
            .expect("Failed to poll device");

        // Read the contents of the read buffer.
        // -------------------------------------

        let buf_slice = self.read_buf.slice(..);

        let finished_flag = Arc::new(AtomicBool::new(false));
        let ff_handle = finished_flag.clone();

        buf_slice.map_async(wgpu::MapMode::Read, move |result| {
            result.expect("Failed to map read buffer");
            ff_handle.store(true, Ordering::SeqCst);
        });

        self.device
            .poll(wgpu::PollType::Wait)
            .expect("Failed to poll device");

        while !finished_flag.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }

        let view = buf_slice.get_mapped_range();
        let data = bytemuck::cast_slice::<_, u32>(&*view).into();

        // Release the read buffer.
        drop(view);
        self.read_buf.unmap();

        data
    }
}
