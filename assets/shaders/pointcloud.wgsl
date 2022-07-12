#import bevy_pbr::mesh_view_bind_group
#import bevy_pbr::mesh_struct

[[group(1), binding(0)]]
var<uniform> mesh: Mesh;

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;

    [[location(3)]] i_pos_scale: vec4<f32>;
    [[location(4)]] i_color: vec4<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let position = vertex.position * vertex.i_pos_scale.w + vertex.i_pos_scale.xyz;
    let world_position = mesh.model * vec4<f32>(position, 1.0);
    let world_normal = (mesh.inverse_transpose_model * vec4<f32>(vertex.normal, 0.0)).xyz;

    var out: VertexOutput;
    out.clip_position = view.view_proj * world_position;
    out.color = vertex.i_color;

    var color = vertex.i_color;
    color = vec4<f32>(color.rgb * (dot(world_normal, normalize(vec3<f32>(0.2, 1.0, 0.1))) * 0.25 + 0.75), color.a);
    // out.color = color;

    // out.color = vec4<f32>(1.0, 0.0, 0.0, 1.0);

    return out;
}

[[stage(fragment)]]
fn fragment(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return in.color;
}
