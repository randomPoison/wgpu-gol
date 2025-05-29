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
    const GRID_SIZE: usize = 8;

    let all_on = [1; GRID_SIZE * GRID_SIZE];

    let mut sim = pollster::block_on(LifeSimulation::new(GRID_SIZE as u32, &all_on));

    let state = sim.read_state();
    assert_grid_eq(GRID_SIZE, &state, &all_on);

    let all_off = [0; GRID_SIZE * GRID_SIZE];
    sim.reset_state(&all_off);

    let state = sim.read_state();
    assert_grid_eq(GRID_SIZE, &state, &all_off);
}

fn still_life() {
    const GRID_SIZE: usize = 8;

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

    let mut sim = pollster::block_on(LifeSimulation::new(GRID_SIZE as u32, &init_state));

    do_step(&mut sim);

    let state = sim.read_state();
    assert_grid_eq(GRID_SIZE, &init_state, &state);

    do_step(&mut sim);

    let state = sim.read_state();
    assert_grid_eq(GRID_SIZE, &init_state, &state);
}

fn glider() {
    const GRID_SIZE: usize = 8;

    let mut sim = pollster::block_on(LifeSimulation::new(GRID_SIZE as u32, &GLIDER_1));

    do_step(&mut sim);
    let state = sim.read_state();
    assert_grid_eq(GRID_SIZE, &state, &GLIDER_2);

    do_step(&mut sim);
    let state = sim.read_state();
    assert_grid_eq(GRID_SIZE, &state, &GLIDER_3);
}

fn big_grid() {
    const GRID_SIZE: usize = 64;

    let mut sim = pollster::block_on(LifeSimulation::new(64, &[0; GRID_SIZE * GRID_SIZE]));

    for x_off in 0..GRID_SIZE - 8 {
        for y_off in 0..GRID_SIZE - 8 {
            eprintln!("Testing with offset ({}, {})", x_off, y_off);

            // Initialize the full grid states by copying the smaller glider patterns into the full buffer.

            let mut big_state_1 = [0u8; GRID_SIZE * GRID_SIZE];
            copy_to_grid(&GLIDER_1, &mut big_state_1, [x_off, y_off]);

            let mut big_state_2 = [0u8; GRID_SIZE * GRID_SIZE];
            copy_to_grid(&GLIDER_2, &mut big_state_2, [x_off, y_off]);

            let mut big_state_3 = [0u8; GRID_SIZE * GRID_SIZE];
            copy_to_grid(&GLIDER_3, &mut big_state_3, [x_off, y_off]);

            // Run the actual test.
            sim.reset_state(&big_state_1);

            do_step(&mut sim);

            let state = sim.read_state();
            assert_grid_eq(GRID_SIZE, &big_state_2, &state);

            do_step(&mut sim);

            let state = sim.read_state();
            assert_grid_eq(GRID_SIZE, &big_state_3, &state);
        }
    }
}

#[track_caller]
fn assert_grid_eq(grid_size: usize, expected: &[u8], actual: &[u8]) {
    assert_eq!(expected.len(), grid_size * grid_size);
    assert_eq!(actual.len(), grid_size * grid_size);

    if expected != actual {
        eprintln!("Grids do not match!");
        eprintln!("Expected grid: [");
        for row in expected.chunks(grid_size) {
            eprintln!("{:?}", row);
        }
        eprintln!("]");

        eprintln!("Actual grid: [");
        for row in actual.chunks(grid_size) {
            eprintln!("{:?}", row);
        }
        eprintln!("]");

        panic!("assert_grid_eq failed: grids do not match");
    }
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
