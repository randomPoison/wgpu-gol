use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use wgpu_gol::LifeSimulation;

const FIXED_GRID_SIZE: u32 = 1024;

fn benchmark(c: &mut Criterion) {
    // Create a random initialize state for the simulation.
    let num_cells = (FIXED_GRID_SIZE * FIXED_GRID_SIZE) as usize;
    let mut init_state = vec![0; num_cells as usize];
    for i in 0..num_cells {
        init_state[i] = rand::random::<u8>() % 2;
    }

    let mut sim = pollster::block_on(LifeSimulation::new(FIXED_GRID_SIZE, &init_state));

    let mut group = c.benchmark_group("Simulate N Steps (1024x1024 Grid)");
    for num_ticks in [1_000 /* 100, 10, 1 */] {
        sim.reset_state(&init_state);

        group.throughput(Throughput::Elements(num_ticks));
        group.bench_with_input(BenchmarkId::from_parameter(num_ticks), &num_ticks, |b, size| {
            b.iter(|| {
                simulate_n_steps(&mut sim, *size);
            });
        });
    }
    group.finish();
    drop(sim);

    let mut group = c.benchmark_group("Simulate NxN Grid (1,000 steps)");
    for size in [256, 512, 1024, 2048, 4096] {
        let num_cells = (size * size) as usize;
        let mut init_state = vec![0; num_cells as usize];
        for i in 0..num_cells {
            init_state[i] = rand::random::<u8>() % 2;
        }

        // TODO: Allow changing the grid size in `reset` so that we can reuse the same
        // simulation instance between benchmarks.
        let mut sim = pollster::block_on(LifeSimulation::new(size, &init_state));

        group.throughput(Throughput::Elements((size * size * 1_000) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _size| {
            b.iter(|| {
                simulate_n_steps(&mut sim, 1000);
            });
        });
    }
    group.finish();
}

fn simulate_n_steps(sim: &mut LifeSimulation, n: u64) {
    sim.step = 0;

    let mut encoder = sim
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

    for _ in 0..n {
        sim.encode_compute_pass(&mut encoder);
    }

    sim.queue.submit([encoder.finish()]);
    sim.device
        .poll(wgpu::PollType::Wait)
        .expect("Failed to poll device");
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
