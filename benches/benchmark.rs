use criterion::{Criterion, criterion_group, criterion_main};
use wgpu_gol::LifeSimulation;

fn benchmark(c: &mut Criterion) {
    let mut sim = pollster::block_on(LifeSimulation::new());

    c.bench_function("1 step", |b| {
        b.iter(|| {
            simulate_n_steps(&mut sim, 1);
        });
    });

    c.bench_function("10 steps", |b| {
        b.iter(|| {
            simulate_n_steps(&mut sim, 10);
        });
    });

    c.bench_function("100 steps", |b| {
        b.iter(|| {
            simulate_n_steps(&mut sim, 100);
        });
    });

    c.bench_function("1000 steps", |b| {
        b.iter(|| {
            simulate_n_steps(&mut sim, 1_000);
        });
    });
}

fn simulate_n_steps(sim: &mut LifeSimulation, n: usize) {
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
