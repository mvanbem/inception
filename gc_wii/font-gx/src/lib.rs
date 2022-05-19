#![no_std]

use gamecube_shader::FLAT_TEXTURED_SHADER;
use ogc_sys::*;

pub struct TextRenderer {
    pub x: u16,
    pub y: u16,
    pub left_margin: u16,
}

impl TextRenderer {
    pub fn prepare(ui_font: &GXTexObj) {
        unsafe {
            GX_ClearVtxDesc();
            GX_SetVtxDesc(GX_VA_POS as u8, GX_DIRECT as u8);
            GX_SetVtxDesc(GX_VA_TEX0 as u8, GX_DIRECT as u8);
            GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_POS, GX_POS_XY, GX_U16, 0);
            GX_SetVtxAttrFmt(GX_VTXFMT0 as u8, GX_VA_TEX0, GX_TEX_ST, GX_U8, 6);
            GX_InvVtxCache();

            FLAT_TEXTURED_SHADER.apply();
            GX_LoadTexObj(
                ui_font as *const GXTexObj as *mut GXTexObj,
                GX_TEXMAP0 as u8,
            );
        }
    }

    pub fn new_line(&mut self) {
        self.x = self.left_margin;
        self.y += 16;
    }

    pub fn draw_char(&mut self, c: u8) {
        if c == b'\n' {
            self.new_line();
            return;
        }

        let x0 = self.x;
        let x1 = x0 + 8;
        let y0 = self.y;
        let y1 = y0 + 16;

        let s0 = ((c & 0xf) << 2) + 1;
        let s1 = s0 + 2;
        let t0 = (c >> 4) << 2;
        let t1 = t0 + 4;

        unsafe {
            GX_Begin(GX_QUADS as u8, GX_VTXFMT0 as u8, 4);

            (*wgPipe).U16 = x0;
            (*wgPipe).U16 = y0;
            (*wgPipe).U8 = s0;
            (*wgPipe).U8 = t0;

            (*wgPipe).U16 = x1;
            (*wgPipe).U16 = y0;
            (*wgPipe).U8 = s1;
            (*wgPipe).U8 = t0;

            (*wgPipe).U16 = x1;
            (*wgPipe).U16 = y1;
            (*wgPipe).U8 = s1;
            (*wgPipe).U8 = t1;

            (*wgPipe).U16 = x0;
            (*wgPipe).U16 = y1;
            (*wgPipe).U8 = s0;
            (*wgPipe).U8 = t1;
        }

        self.x += 8;
        if self.x + 8 > 640 {
            self.new_line();
        }
    }

    pub fn draw_str(&mut self, s: &[u8]) {
        for &c in s {
            self.draw_char(c);
        }
    }
}
