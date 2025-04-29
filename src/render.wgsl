@group(0) @binding(0) var<uniform> grid_size: vec2f;

@vertex
fn vertex_main(@location(0) pos: vec2f) -> @builtin(position) vec4f {
    let cell_coords = vec2f(0, 0);

    // Calculate the size of a cell in clip space.
    let cell_size = 2.0 / grid_size;

    // Divide by the grid size so that our square is the correct size.
    var grid_pos = pos / grid_size;

    // Shift the square so that its top left corner is centered in the window.
    grid_pos += (cell_size / 2f) * vec2f(1, -1);

    // Shift the square so that its top left corner is in the top left corner of the window.
    grid_pos += vec2f(-1, 1);

    return vec4f(grid_pos, 0, 1);
}

@fragment
fn fragment_main() -> @location(0) vec4f {
    return vec4f(1, 0, 0, 1);
}
