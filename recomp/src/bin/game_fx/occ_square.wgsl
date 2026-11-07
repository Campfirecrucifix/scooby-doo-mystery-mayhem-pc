// P33 R3: RenderSpriteNewOcc pass 2 (game_fx::OccSquare): where the sample square passes the Z test
// the frame alpha becomes 0x80; the port writes green = 1 into the stencil-mask image (write mask G).
#import bevy_pbr::forward_io::VertexOutput

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 1.0, 0.0, 1.0);
}
