// Edge line shader — instanced rendering of oriented quads.
//
// Each edge is a unit quad stretched and rotated between two node positions.
// The vertex shader computes the quad corners from start/end/thickness.
// The fragment shader applies smooth edges via SDF on the short axis.

// ───────────────────── Uniform ─────────────────────

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

// ───────────────────── Vertex ─────────────────────

struct VertexInput {
    // Base quad vertex (slot 0) — only x used: -1 or +1 for the two ends
    @location(0) quad_pos: vec2<f32>,
    // Per-instance data (slot 1)
    @location(2) start: vec2<f32>,
    @location(3) end: vec2<f32>,
    @location(4) thickness: f32,
    @location(5) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_y: f32,  // -1..1 across the line width (for AA)
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Direction vector from start to end.
    let dir = in.end - in.start;
    let len = length(dir);

    // Avoid division by zero for zero-length edges.
    if len < 0.001 {
        out.clip_position = vec4<f32>(0.0, 0.0, -2.0, 1.0); // off-screen
        out.local_y = 0.0;
        out.color = in.color;
        return out;
    }

    // Normalized direction and perpendicular.
    let n_dir = dir / len;
    let perp = vec2<f32>(-n_dir.y, n_dir.x);

    // quad_pos.x ∈ {-1, 1}: selects start or end of the line.
    // quad_pos.y ∈ {-1, 1}: selects the two sides (perpendicular offset).
    let t = (in.quad_pos.x + 1.0) * 0.5; // 0..1
    let along = mix(in.start, in.end, vec2<f32>(t, t));
    let offset = perp * in.thickness * 0.5 * in.quad_pos.y;

    let world_pos = along + offset;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 0.0, 1.0);
    out.local_y = in.quad_pos.y; // -1..1 across line width
    out.color = in.color;

    return out;
}

// ───────────────────── Fragment ─────────────────────

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Smooth edges on the perpendicular axis.
    let dist = abs(in.local_y);
    let fw = fwidth(dist);
    let alpha = 1.0 - smoothstep(1.0 - fw * 2.0, 1.0, dist);

    if alpha < 0.01 {
        discard;
    }

    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
