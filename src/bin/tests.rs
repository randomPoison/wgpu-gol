use wgpu_gol::LifeSimulation;

fn zero_steps() {
    let all_on = [1; 64];

    let mut sim = pollster::block_on(LifeSimulation::new(8, &all_on));

    let state = sim.read_state();
    assert_eq!(&state, &all_on);

    let all_off = [0; 64];
    sim.reset_state(&all_off);

    let state = sim.read_state();
    assert_eq!(&state, &all_off);
}

fn still_life() {
    #[rustfmt::skip]
    let init_state = [
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 1, 1, 0, 0, 0, 0, 0,
        0, 1, 1, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 1, 1, 0,
        0, 0, 0, 0, 1, 0, 1, 0,
        0, 0, 0, 0, 1, 1, 0, 0,
    ];

    let mut sim = pollster::block_on(LifeSimulation::new(8, &init_state));

    do_step(&mut sim);

    let state = sim.read_state();
    assert_eq!(&state, &init_state);

    do_step(&mut sim);

    let state = sim.read_state();
    assert_eq!(&state, &init_state);
}

fn glider() {
    #[rustfmt::skip]
    let state1 = [
        0, 0, 1, 0, 0, 0, 0, 0,
        1, 0, 1, 0, 0, 0, 0, 0,
        0, 1, 1, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
    ];


    #[rustfmt::skip]
    let state2 = [
        0, 1, 0, 0, 0, 0, 0, 0,
        0, 0, 1, 1, 0, 0, 0, 0,
        0, 1, 1, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
    ];

    #[rustfmt::skip]
    let state3 = [
        0, 0, 1, 0, 0, 0, 0, 0,
        0, 0, 0, 1, 0, 0, 0, 0,
        0, 1, 1, 1, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
    ];

    let mut sim = pollster::block_on(LifeSimulation::new(8, &state1));

    do_step(&mut sim);
    let state = sim.read_state();
    assert_eq!(&state, &state2);

    do_step(&mut sim);
    let state = sim.read_state();
    assert_eq!(&state, &state3);
}

fn do_step(sim: &mut LifeSimulation) {
    let mut encoder = sim
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

    sim.encode_compute_pass(&mut encoder);

    sim.queue.submit([encoder.finish()]);
    sim.device
        .poll(wgpu::PollType::Wait)
        .expect("Failed to poll device");
}

fn main() {
    zero_steps();
    still_life();
    glider();
}
