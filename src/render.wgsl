@group(0) @binding(0) var<uniform> grid_size: vec2f;
@group(0) @binding(1) var<storage> in_state: array<u32>;

@vertex
fn vertex_main(
    @location(0) pos: vec2f,
    @builtin(instance_index) instance_index: u32,
) -> @builtin(position) vec4f {
    // Calculate the coordinates of the cell in the grid.
    let index = f32(instance_index);
    let cell_coords = vec2f(floor(index / grid_size.x), index % grid_size.x);

    // Calculate the size of a cell in clip space.
    let cell_size = 2.0 / grid_size;

    // Divide by the grid size so that our square is the correct size.
    var grid_pos = pos / grid_size;

    // Shift the square so that its top left corner is centered in the window.
    grid_pos += (cell_size / 2f) * vec2f(1, -1);

    // Shift the square so that its top left corner is in the top left corner of the window.
    grid_pos += vec2f(-1, 1);

    // Shift the square to the position for its cell coordinates.
    grid_pos += cell_coords * cell_size * vec2f(1, -1);

    // Scale the square to 0 if the cell is disabled.
    grid_pos *= f32(in_state[instance_index]);

    return vec4f(grid_pos, 0, 1);
}

@fragment
fn fragment_main() -> @location(0) vec4f {
    return vec4f(1, 0, 0, 1);
}
