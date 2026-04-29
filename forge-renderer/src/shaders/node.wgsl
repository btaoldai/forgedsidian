// Node circle shader — instanced rendering with SDF anti-aliased circles.
//
// Each node is a unit quad [-1, 1] transformed by the vertex shader to
// the correct position and size. The fragment shader uses a signed distance
// function to create a perfect circle with smooth anti-aliased edges.

// ───────────────────── Uniform ─────────────────────

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

// ───────────────────── Vertex ─────────────────────

struct VertexInput {
    // Base quad vertex (slot 0)
    @location(0) quad_pos: vec2<f32>,
    // Per-instance data (slot 1)
    @location(2) center: vec2<f32>,
    @location(3) radius: f32,
    @location(4) color: vec4<f32>,
    @location(5) pick_id: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,  // -1..1 within the quad
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Scale quad by radius and translate to node center (graph-space).
    let world_pos = in.center + in.quad_pos * in.radius;

    // Project to clip-space.
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 0.0, 1.0);

    // Pass local quad coordinate for SDF in fragment shader.
    out.local_pos = in.quad_pos;
    out.color = in.color;

    return out;
}

// ───────────────────── Fragment ─────────────────────

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Signed distance from center of the quad (0,0).
    // At the edge of the circle, dist = 1.0.
    let dist = length(in.local_pos);

    // Discard pixels outside the circle (with AA).
    // fwidth gives the rate of change per pixel — used for smooth edges.
    let fw = fwidth(dist);
    let alpha = 1.0 - smoothstep(1.0 - fw, 1.0 + fw, dist);

    if alpha < 0.01 {
        discard;
    }

    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}

// ───────────────── Pick Pass Fragment ─────────────────

// Encodes the node's pick_id as a color for GPU picking.
// The CPU reads back this color to determine which node was clicked.

struct PickVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) @interpolate(flat) pick_id: u32,
};

@vertex
fn vs_pick(in: VertexInput) -> PickVertexOutput {
    var out: PickVertexOutput;
    let world_pos = in.center + in.quad_pos * in.radius;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 0.0, 1.0);
    out.local_pos = in.quad_pos;
    out.pick_id = in.pick_id;
    return out;
}

@fragment
fn fs_pick(in: PickVertexOutput) -> @location(0) vec4<u32> {
    let dist = length(in.local_pos);
    if dist > 1.0 {
        discard;
    }
    // Encode pick_id into RGBA (R = id, G/B/A = 0 for now).
    // For >16M nodes, spread across R+G+B channels.
    return vec4<u32>(in.pick_id, 0u, 0u, 255u);
}
