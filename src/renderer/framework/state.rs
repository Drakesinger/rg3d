use crate::{
    renderer::{
        framework::{
            framebuffer::{
                CullFace,
                DrawParameters,
            },
            gl::{
                self,
                types::{
                    GLuint,
                    GLboolean,
                    GLenum,
                    GLint
                }
            }
        }
    },
    core::{
        math::Rect,
        color::Color,
    },
};

pub struct State {
    blend: bool,
    depth_test: bool,
    depth_write: bool,
    color_write: ColorMask,
    stencil_test: bool,
    cull_face: CullFace,
    culling: bool,
    stencil_mask: u32,
    clear_color: Color,
    clear_stencil: i32,
    clear_depth: f32,

    framebuffer: GLuint,
    viewport: Rect<i32>,

    blend_src_factor: GLuint,
    blend_dst_factor: GLuint,

    program: GLuint,
    texture_units: [TextureUnit; 32],

    stencil_func: StencilFunc,
    stencil_op: StencilOp,
}

#[derive(Copy, Clone)]
struct TextureUnit {
    target: GLenum,
    texture: GLuint,
}

impl Default for TextureUnit {
    fn default() -> Self {
        Self {
            target: gl::TEXTURE_2D,
            texture: 0,
        }
    }
}

fn bool_to_gl_bool(v: bool) -> GLboolean {
    if v {
        gl::TRUE
    } else {
        gl::FALSE
    }
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Hash, Debug)]
pub struct ColorMask {
    red: bool,
    green: bool,
    blue: bool,
    alpha: bool,
}

impl Default for ColorMask {
    fn default() -> Self {
        Self {
            red: true,
            green: true,
            blue: true,
            alpha: true,
        }
    }
}

impl ColorMask {
    pub fn all(value: bool) -> Self {
        Self {
            red: value,
            green: value,
            blue: value,
            alpha: value,
        }
    }
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Hash, Debug)]
pub struct StencilFunc {
    pub func: GLenum,
    pub ref_value: GLint,
    pub mask: GLuint,
}

impl Default for StencilFunc {
    fn default() -> Self {
        Self {
            func: gl::ALWAYS,
            ref_value: 0,
            mask: 0xFFFF_FFFF,
        }
    }
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Hash, Debug)]
pub struct StencilOp {
    pub fail: GLenum,
    pub zfail: GLenum,
    pub zpass: GLenum,
}

impl Default for StencilOp {
    fn default() -> Self {
        Self {
            fail: gl::KEEP,
            zfail: gl::KEEP,
            zpass: gl::KEEP,
        }
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            blend: false,
            depth_test: false,
            depth_write: true,
            color_write: Default::default(),
            stencil_test: false,
            cull_face: CullFace::Back,
            culling: false,
            stencil_mask: 0xFFFF_FFFF,
            clear_color: Color::from_rgba(0, 0, 0, 0),
            clear_stencil: 0,
            clear_depth: 1.0,
            framebuffer: 0,
            viewport: Rect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
            blend_src_factor: gl::ONE,
            blend_dst_factor: gl::ZERO,
            program: 0,
            texture_units: [Default::default(); 32],
            stencil_func: Default::default(),
            stencil_op: Default::default(),
        }
    }

    pub fn set_framebuffer(&mut self, framebuffer: GLuint) {
        if self.framebuffer != framebuffer {
            self.framebuffer = framebuffer;

            unsafe {
                gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer)
            }
        }
    }

    pub fn set_viewport(&mut self, viewport: Rect<i32>) {
        if self.viewport != viewport {
            self.viewport = viewport;

            unsafe {
                gl::Viewport(self.viewport.x, self.viewport.y, self.viewport.w, self.viewport.h);
            }
        }
    }

    pub fn set_blend(&mut self, blend: bool) {
        if self.blend != blend {
            self.blend = blend;

            unsafe {
                if self.blend {
                    gl::Enable(gl::BLEND);
                } else {
                    gl::Disable(gl::BLEND);
                }
            }
        }
    }

    pub fn set_depth_test(&mut self, depth_test: bool) {
        if self.depth_test != depth_test {
            self.depth_test = depth_test;

            unsafe {
                if self.depth_test {
                    gl::Enable(gl::DEPTH_TEST);
                } else {
                    gl::Disable(gl::DEPTH_TEST);
                }
            }
        }
    }

    pub fn set_depth_write(&mut self, depth_write: bool) {
        if self.depth_write != depth_write {
            self.depth_write = depth_write;

            unsafe {
                gl::DepthMask(bool_to_gl_bool(self.depth_write));
            }
        }
    }

    pub fn set_color_write(&mut self, color_write: ColorMask) {
        if self.color_write != color_write {
            self.color_write = color_write;

            unsafe {
                gl::ColorMask(bool_to_gl_bool(self.color_write.red),
                              bool_to_gl_bool(self.color_write.green),
                              bool_to_gl_bool(self.color_write.blue),
                              bool_to_gl_bool(self.color_write.alpha));
            }
        }
    }

    pub fn set_stencil_test(&mut self, stencil_test: bool) {
        if self.stencil_test != stencil_test {
            self.stencil_test = stencil_test;

            unsafe {
                if self.stencil_test {
                    gl::Enable(gl::STENCIL_TEST);
                } else {
                    gl::Disable(gl::STENCIL_TEST);
                }
            }
        }
    }

    pub fn set_cull_face(&mut self, cull_face: CullFace) {
        if self.cull_face != cull_face {
            self.cull_face = cull_face;

            unsafe {
                gl::CullFace(self.cull_face.into_gl_value())
            }
        }
    }

    pub fn set_culling(&mut self, culling: bool) {
        if self.culling != culling {
            self.culling = culling;

            unsafe {
                if self.culling {
                    gl::Enable(gl::CULL_FACE);
                } else {
                    gl::Disable(gl::CULL_FACE);
                }
            }
        }
    }

    pub fn set_stencil_mask(&mut self, stencil_mask: u32) {
        if self.stencil_mask != stencil_mask {
            self.stencil_mask = stencil_mask;

            unsafe {
                gl::StencilMask(stencil_mask);
            }
        }
    }

    pub fn set_clear_color(&mut self, color: Color) {
        if self.clear_color != color {
            self.clear_color = color;

            let rgba = color.as_frgba();
            unsafe {
                gl::ClearColor(rgba.x, rgba.y, rgba.z, rgba.w);
            }
        }
    }

    pub fn set_clear_depth(&mut self, depth: f32) {
        if self.clear_depth != depth {
            self.clear_depth = depth;

            unsafe {
                gl::ClearDepth(depth as f64);
            }
        }
    }

    pub fn set_clear_stencil(&mut self, stencil: i32) {
        if self.clear_stencil != stencil {
            self.clear_stencil = stencil;

            unsafe {
                gl::ClearStencil(stencil);
            }
        }
    }

    pub fn set_blend_func(&mut self, sfactor: GLenum, dfactor: GLenum) {
        if self.blend_src_factor != sfactor || self.blend_dst_factor != dfactor {
            self.blend_src_factor = sfactor;
            self.blend_dst_factor = dfactor;

            unsafe {
                gl::BlendFunc(self.blend_src_factor, self.blend_dst_factor);
            }
        }
    }

    pub fn set_program(&mut self, program: GLuint) {
        if self.program != program {
            self.program = program;

            unsafe {
                gl::UseProgram(self.program);
            }
        }
    }

    pub fn set_texture(&mut self, sampler_index: usize, target: GLenum, texture: GLuint) {
        let unit = self.texture_units.get_mut(sampler_index).unwrap();

        if unit.target != target || unit.texture != texture {
            unit.texture = texture;
            unit.target = target;

            unsafe {
                gl::ActiveTexture(gl::TEXTURE0 + sampler_index as u32);
                gl::BindTexture(target, texture);
            }
        }
    }

    pub fn set_stencil_func(&mut self, func: StencilFunc) {
        if self.stencil_func != func {
            self.stencil_func = func;

            unsafe {
                gl::StencilFunc(self.stencil_func.func, self.stencil_func.ref_value, self.stencil_func.mask);
            }
        }
    }

    pub fn set_stencil_op(&mut self, op: StencilOp) {
        if self.stencil_op != op {
            self.stencil_op = op;

            unsafe {
                gl::StencilOp(self.stencil_op.fail, self.stencil_op.zfail, self.stencil_op.zpass);
            }
        }
    }

    pub fn invalidate_resource_bindings_cache(&mut self) {
        self.texture_units = Default::default();
        self.program = 0;
    }

    pub fn apply_draw_parameters(&mut self, draw_params: &DrawParameters) {
        self.set_blend(draw_params.blend);
        self.set_depth_test(draw_params.depth_test);
        self.set_depth_write(draw_params.depth_write);
        self.set_color_write(draw_params.color_write);
        self.set_stencil_test(draw_params.stencil_test);
        self.set_cull_face(draw_params.cull_face);
        self.set_culling(draw_params.culling);
    }
}