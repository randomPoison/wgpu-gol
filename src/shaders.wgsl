@group(0) @binding(0) var<uniform> grid_size: vec2f;
@group(0) @binding(3) var<uniform> physical_grid_size: vec2u;
@group(0) @binding(1) var<storage> in_state: array<u32>;
@group(0) @binding(2) var<storage, read_write> out_state: array<u32>;

// TODO: Inject the workgroup size at runtime?
@compute @workgroup_size(64)
fn compute_main(@builtin(global_invocation_id) invocation: vec3u) {
    let block_index = invocation.x;

    // If the number of blocks isn't a clean multiple of the workgroup size we
    // end up with extra invocations that don't correspond to a real block. We
    // can simply do nothing in that case.
    //
    // TODO: Is this even necessary? Tests all pass when I remove the early
    // return, but I'm not sure what the exact semantics of reading past the end
    // of `in_state` or writing past the end of `out_state` are, so I'm not sure
    // if removing this would cause some subtle bug I'm not covering in the
    // tests.
    if (block_index >= physical_grid_size.x * physical_grid_size.y) {
        return;
    }

    // TODO: Make this a uniform.
    let cells_per_row = min(physical_grid_size.x * 32, u32(grid_size.x));

    let block_row = block_index / physical_grid_size.x;
    let block_col = block_index % physical_grid_size.x;
    let block_start_cell = block_row * cells_per_row + block_col * 32;

    let block_in = in_state[block_index];
    var block_out = 0u;

    for (var bit_index = 0u; bit_index < 32u; bit_index++) {
        let cell_index = block_start_cell + bit_index;
        let cell = cell_index_to_cell_coords(cell_index);

        // Skip cells that are outside the grid.
        //
        // TODO: It would be better to cap bit_index at the appropriate max value.
        if (cell.x >= u32(grid_size.x)) {
            break;
        }

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

        let mask = 1u << bit_index;

        // Conway's game of life rules:
        switch active_neighbors {
            // Active cells with 2 neighbors stay active.
            case 2: {
                block_out |= block_in & mask;
            }

            // Cells with 3 neighbors become or stay active.
            case 3: {
                block_out |= mask;
            }

            // Cells with < 2 or > 3 neighbors become inactive.
            default: {
                block_out &= ~mask;
            }
        }
    }

    out_state[block_index] = block_out;
}

// Converts the cell coordinates into block coordinates (i.e. block index and
// bit index).
//
// TODO: Create a struct for the block coords to make the block index vs bit
// index more clear.
fn block_index(cell: vec2u) -> vec2u {
    let wrapped_coords = vec2u(
        cell.x % u32(grid_size.x),
        cell.y % u32(grid_size.y),
    );

    let block_index =
        physical_grid_size.x * wrapped_coords.y + wrapped_coords.x / 32;
    let bit_index = cell.x % 32;
    return vec2u(block_index, bit_index);
}

fn cell_active(x: u32, y: u32) -> u32 {
    let block_coords = block_index(vec2u(x, y));
    let block_index = block_coords.x;
    let bit_index = block_coords.y;

    let mask = 1u << bit_index;
    return u32((in_state[block_index] & mask) != 0u);
}

fn cell_index_to_cell_coords(index: u32) -> vec2u {
    // TODO: Extract grid width to a uniform.
    let width = u32(grid_size.x);
    return vec2u(index % width, index / width);
}

// =============================================================================
// Rendering
// =============================================================================

@vertex
fn vertex_main(
    @location(0) pos: vec2f,
    @builtin(instance_index) cell_index: u32,
) -> @builtin(position) vec4f {
    // Calculate the coordinates of the cell in the grid.
    let cell_coords = cell_index_to_cell_coords(cell_index);

    // Calculate the size of a cell in clip space.
    let cell_size = 2.0 / grid_size;

    // Divide by the grid size so that our square is the correct size.
    var grid_pos = pos / grid_size;

    // Shift the square so that its top left corner is centered in the window.
    grid_pos += (cell_size / 2f) * vec2f(1, -1);

    // Shift the square so that its top left corner is in the top left corner of the window.
    grid_pos += vec2f(-1, 1);

    // Shift the square to the position for its cell coordinates.
    grid_pos += vec2f(cell_coords) * cell_size * vec2f(1, -1);

    // Scale the square to 0 if the cell is disabled.
    grid_pos *= f32(cell_active(
        cell_coords.x,
        cell_coords.y,
    ));

    return vec4f(grid_pos, 0, 1);
}

@fragment
fn fragment_main() -> @location(0) vec4f {
    return vec4f(1, 0, 0, 1);
}
