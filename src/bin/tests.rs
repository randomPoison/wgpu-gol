use wgpu_gol::LifeSimulation;

#[rustfmt::skip]
static GLIDER_1: &[u8] = &[
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
static GLIDER_2: &[u8] = &[
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
static GLIDER_3: &[u8] = &[
    0, 0, 1, 0, 0, 0, 0, 0,
    0, 0, 0, 1, 0, 0, 0, 0,
    0, 1, 1, 1, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
];

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
    let mut sim = pollster::block_on(LifeSimulation::new(8, &GLIDER_1));

    do_step(&mut sim);
    let state = sim.read_state();
    assert_eq!(&state, &GLIDER_2);

    do_step(&mut sim);
    let state = sim.read_state();
    assert_eq!(&state, &GLIDER_3);
}

fn big_grid() {
    const GRID_SIZE: usize = 64;

    // Initialize the full grid states by copying the smaller glider patterns into the full buffer.

    let mut big_state_1 = [0u8; GRID_SIZE * GRID_SIZE];
    copy_to_grid(&GLIDER_1, &mut big_state_1, [0, 0]);

    let mut big_state_2 = [0u8; GRID_SIZE * GRID_SIZE];
    copy_to_grid(&GLIDER_2, &mut big_state_2, [0, 0]);

    let mut big_state_3 = [0u8; GRID_SIZE * GRID_SIZE];
    copy_to_grid(&GLIDER_3, &mut big_state_3, [0, 0]);

    // Run the actual test.

    let mut sim = pollster::block_on(LifeSimulation::new(64, &big_state_1));

    do_step(&mut sim);

    let state = sim.read_state();
    assert_eq!(&state, &big_state_2);

    do_step(&mut sim);

    let state = sim.read_state();
    assert_eq!(&state, &big_state_3);
}

/// Copies a smaller 8x8 grid into a larger 64x64 grid at the specified offset.
fn copy_to_grid(src: &[u8], dst: &mut [u8], offset: [usize; 2]) {
    // For now we're hard coding the expected size of the src and dst buffers. This
    // is fine for now because we're only using it to test a 64x64 grid.
    assert_eq!(src.len(), 8 * 8);
    assert_eq!(dst.len(), 64 * 64);

    let [x_offset, y_offset] = offset;

    for row in 0..8 {
        let dst_row = row + y_offset;
        let dst = &mut dst[dst_row * 64 + x_offset..][..8];
        dst.copy_from_slice(&src[row * 8..][..8]);
    }
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
    big_grid();
}
