use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use wgpu_gol::LifeSimulation;

const GRID_SIZE: u32 = 1024;

fn benchmark(c: &mut Criterion) {
    let mut sim = pollster::block_on(LifeSimulation::new(GRID_SIZE));

    let mut group = c.benchmark_group("Simulate N Steps");
    for size in [1_u64, 10, 100, 1_000] {
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
