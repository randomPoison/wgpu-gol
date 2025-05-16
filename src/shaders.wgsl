@group(0) @binding(0) var<uniform> grid_size: vec2f;
@group(0) @binding(1) var<storage> in_state: array<u32>;
@group(0) @binding(2) var<storage, read_write> out_state: array<u32>;

// TODO: Inject the workgroup size at runtime?
@compute @workgroup_size(8, 8)
fn compute_main(@builtin(global_invocation_id) cell: vec3u) {
  // Determine how many active neighbors this cell has.
  let active_neighbors =
    cell_active(cell.x + 1, cell.y + 1) +
    cell_active(cell.x + 1, cell.y    ) +
    cell_active(cell.x + 1, cell.y - 1) +
    cell_active(cell.x,     cell.y - 1) +
    cell_active(cell.x - 1, cell.y - 1) +
    cell_active(cell.x - 1, cell.y    ) +
    cell_active(cell.x - 1, cell.y + 1) +
    cell_active(cell.x,     cell.y + 1);

    let i = cell_index(cell.xy);

    // Conway's game of life rules:
    switch active_neighbors {
        // Active cells with 2 neighbors stay active.
        case 2: {
            out_state[i] = in_state[i];
        }

        // Cells with 3 neighbors become or stay active.
        case 3: {
            out_state[i] = 1;
        }

        // Cells with < 2 or > 3 neighbors become inactive.
        default: {
            out_state[i] = 0;
        }
    }
}

fn cell_index(cell: vec2u) -> u32 {
    return (cell.y % u32(grid_size.y)) * u32(grid_size.x) +
        (cell.x % u32(grid_size.x));
}

fn cell_active(x: u32, y: u32) -> u32 {
    return in_state[cell_index(vec2(x, y))];
}

// =============================================================================
// Rendering
// =============================================================================

@vertex
fn vertex_main(
    @location(0) pos: vec2f,
    @builtin(instance_index) instance_index: u32,
) -> @builtin(position) vec4f {
    // Calculate the coordinates of the cell in the grid.
    let index = f32(instance_index);
    let cell_coords = vec2f(index % grid_size.x, floor(index / grid_size.x));

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
    grid_pos *= f32(cell_active(
        u32(cell_coords.x),
        u32(cell_coords.y),
    ));

    return vec4f(grid_pos, 0, 1);
}

@fragment
fn fragment_main() -> @location(0) vec4f {
    return vec4f(1, 0, 0, 1);
}
