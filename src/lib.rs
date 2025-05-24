use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
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

    /// The size in **cells** of the grid. This will be different from the
    /// number of blocks in the grid.
    pub logical_grid_size: u32,

    /// The total number of cells in the grid.
    ///
    /// Always `logical_grid_size` squared.
    pub num_cells: usize,

    /// The size in **blocks** of the grid. This will not be square because the
    /// individual blocks represent rectangular chunks of the grid.
    pub physical_grid_size: [u32; 2],

    /// The total number of `u32` blocks in state buffers.
    ///
    /// Always `physical_grid_size[0] * physical_grid_size[1]`.
    pub num_blocks: u32,
}

impl LifeSimulation {
    pub async fn new(grid_size: u32, initial_state: &[u8]) -> Self {
        // Make sure the grid size is a multiple of the workgroup size so that
        // the simulation can be broken up cleanly into workgroups.
        assert!(
            grid_size as u32 % WORKGROUP_SIZE == 0,
            "Grid size {grid_size} is not divisible by {WORKGROUP_SIZE}",
        );

        let num_cells = (grid_size * grid_size) as usize;

        // Make sure the initial state is the right size.
        assert!(
            initial_state.len() == num_cells,
            "Initial state has wrong size, expected {} but got {}",
            num_cells,
            initial_state.len(),
        );

        // Convert the list of bytes to a list of u32s.
        let (packed_state, physical_grid_size) = pack_grid(grid_size, initial_state);
        let num_blocks = physical_grid_size[0] * physical_grid_size[1];

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

        let physical_grid_size_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Physical Grid Size Buffer"),
                contents: bytemuck::cast_slice(&physical_grid_size),
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
                    binding: 3,
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

        let cell_state_buffer_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cell State Buffer A"),
            contents: bytemuck::cast_slice(&packed_state),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        let empty_state = vec![0u32; num_blocks as usize];
        let cell_state_buffer_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cell State Buffer B"),
            contents: bytemuck::cast_slice(&empty_state),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        let bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind Group A"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: grid_size_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: physical_grid_size_buffer.as_entire_binding(),
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
            label: Some("Bind Group B"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: grid_size_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: physical_grid_size_buffer.as_entire_binding(),
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
            contents: bytemuck::cast_slice(&empty_state),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader =
            std::fs::read_to_string("src/shaders.wgsl").expect("Failed to read shader file");
        let simulation_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Simulation Shader"),
            source: wgpu::ShaderSource::Wgsl(shader.into()),
        });
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
            logical_grid_size: grid_size,
            num_cells,
            physical_grid_size,
            num_blocks,
        }
    }

    // Restarts the simulation
    pub fn reset_state(&mut self, state: &[u8]) {
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

        // Convert the list of bytes to a list of u32s.
        let in_state = pack_grid(self.logical_grid_size, state).0;

        self.queue
            .write_buffer(&self.state_bufs[0], 0, bytemuck::cast_slice(&in_state));
    }

    pub fn encode_compute_pass(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.bind_groups[(self.step % 2) as usize], &[]);
        compute_pass.dispatch_workgroups(
            self.logical_grid_size / WORKGROUP_SIZE,
            self.logical_grid_size / WORKGROUP_SIZE,
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
        encoder.copy_buffer_to_buffer(
            src_buffer,
            0,
            &self.read_buf,
            0,
            (self.num_blocks as usize * size_of::<u32>()) as u64,
        );
    }

    /// Reads the current grid state from the GPU, blocking until the read
    /// completes.
    pub fn read_state(&self) -> Vec<u8> {
        // Have the GPU copy the current state to the read buffer.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Read State Encoder"),
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
        let raw_data = bytemuck::cast_slice::<_, u32>(&*view);

        // Convert the raw data to a byte array.
        let byte_grid = unpack_grid(self.logical_grid_size, raw_data);

        // Release the read buffer.
        drop(view);
        self.read_buf.unmap();

        byte_grid
    }
}

/// Calcuate the size of the logical grid, and packs the initial state into a
/// vector of `u32`s.
pub fn pack_grid(grid_size: u32, initial_state: &[u8]) -> (Vec<u32>, [u32; 2]) {
    assert_eq!(initial_state.len(), (grid_size * grid_size) as usize);

    // Calculate the width and height in blocks.
    let block_width = grid_size.div_ceil(32);
    let block_height = grid_size;
    let num_blocks = block_width * block_height;

    let mut packed_state = vec![0u32; num_blocks as usize];
    for x in 0..grid_size {
        for y in 0..grid_size {
            let cell_index = y * grid_size + x;
            let state = initial_state[cell_index as usize] as u32;

            let block_index = block_width * y + x / 32;
            let bit_index = x % 32;
            let mask = state << bit_index;

            let block = &mut packed_state[block_index as usize];
            *block |= mask;
        }
    }

    (packed_state, [block_width, block_height])
}

pub fn unpack_grid(grid_size: u32, packed_state: &[u32]) -> Vec<u8> {
    let mut unpacked_state = vec![0u8; (grid_size * grid_size) as usize];

    let block_width = grid_size.div_ceil(32);

    for x in 0..grid_size {
        for y in 0..grid_size {
            let cell_index = y * grid_size + x;

            let block_index = block_width * y + x / 32;
            let bit_index = x % 32;
            let mask = 1 << bit_index;
            let state = (packed_state[block_index as usize] & mask) != 0;

            unpacked_state[cell_index as usize] = state as u8;
        }
    }

    unpacked_state
}
