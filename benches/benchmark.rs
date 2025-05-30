use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use wgpu_gol::LifeSimulation;

const GRID_SIZE: u32 = 1024;

fn benchmark(c: &mut Criterion) {
    // Create a random initialize state for the simulation.
    let num_cells = (GRID_SIZE * GRID_SIZE) as usize;
    let mut init_state = vec![0; num_cells as usize];
    for i in 0..num_cells {
        init_state[i] = rand::random::<u8>() % 2;
    }

    let mut sim = pollster::block_on(LifeSimulation::new(GRID_SIZE, &init_state));

    let mut group = c.benchmark_group("Simulate N Steps");
    for size in [1_000, 100, 10, 1] {
        group.throughput(Throughput::Elements(size));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, size| {
            b.iter(|| {
                simulate_n_steps(&mut sim, *size);
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
