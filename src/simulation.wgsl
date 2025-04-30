@group(0) @binding(0) var<uniform> grid_size: vec2f;

@group(0) @binding(1) var<storage> in_state: array<u32>;
@group(0) @binding(2) var<storage, read_write> out_state: array<u32>;

// TODO: Inject the workgroup size at runtime?
@compute @workgroup_size(8, 8)
fn compute_main(@builtin(global_invocation_id) global_id: vec3u) {
    let cell_index = cell_index(global_id.xy);
    if (in_state[cell_index] == 1) {
        out_state[cell_index] = 0;
    } else {
        out_state[cell_index] = 1;
    }
}

fn cell_index(cell: vec2u) -> u32 {
    return cell.y * u32(grid_size.x) + cell.x;
}
