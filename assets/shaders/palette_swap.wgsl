// Since post processing is a fullscreen effect, we use the fullscreen vertex shader provided by bevy.
// This will import a vertex shader that renders a single fullscreen triangle.
//
// A fullscreen triangle is a single triangle that covers the entire screen.
// The box in the top left in that diagram is the screen. The 4 x are the corner of the screen
//
// Y axis
//  1 |  x-----x......
//  0 |  |  s  |  . ´
// -1 |  x_____x´
// -2 |  :  .´
// -3 |  :´
//    +---------------  X axis
//      -1  0  1  2  3
//
// As you can see, the triangle ends up bigger than the screen.
//
// You don't need to worry about this too much since bevy will compute the correct UVs for you.
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
struct PostProcessSettings {
     // The four colours in our palette
    colours: array<vec3<f32>, 4>,
    darkness: i32,

#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 structs must be 16 byte aligned.
    _webgl2_padding: vec3<f32>
#endif
}
@group(0) @binding(2) var<uniform> settings: PostProcessSettings;

fn get_palette_colour(index: i32) -> vec3<f32> {
    var darkness_mod = clamp(index + settings.darkness, 0, 3);

    return settings.colours[darkness_mod];
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // Determine which palette colour we are going to use
    let colour_r = textureSample(screen_texture, texture_sampler, in.uv).r;
    if colour_r > 0.75 {
       return vec4<f32>(get_palette_colour(0), 1.0);
    } else if colour_r > 0.3 {
        return vec4<f32>(get_palette_colour(1), 1.0);
    } else if colour_r > 0.1 {
        return vec4<f32>(get_palette_colour(2), 1.0);
    } else {
        return vec4<f32>(get_palette_colour(3), 1.0);
    }

    // fallback
    return vec4<f32>(colour_r, colour_r, colour_r, 1.0);
}
