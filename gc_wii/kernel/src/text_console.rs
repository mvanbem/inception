use core::fmt::{self, Write};
use core::mem::size_of;

use array_const_fn_init::array_const_fn_init;
use gamecube_cpu::cache::flush_data_cache_block;
use gamecube_video_driver::framebuffer::Framebuffer;

// Pixel dimensions.
const CELL_WIDTH: usize = 8;
const CELL_HEIGHT: usize = 16;
const SCREEN_WIDTH: usize = 640;
const SCREEN_HEIGHT: usize = 480;

// The XFB pixel format encodes two pixels in a u32.
const WORDS_PER_SCANLINE: usize = SCREEN_WIDTH / 2;

const CACHE_LINE_BYTES: usize = 32;
const CACHE_LINE_WORDS: usize = 8;

#[repr(transparent)]
pub struct Font([[[u8; CELL_WIDTH]; CELL_HEIGHT]; 256]);

impl Font {
    pub const fn from_slice(data: &[u8]) -> &Self {
        assert!(data.len() == size_of::<Font>());
        // SAFETY: Font is repr(transparent) to a byte array.
        unsafe { &*data.as_ptr().cast() }
    }
}

static FONT: &Font = Font::from_slice(include_bytes!("../../../build/console_font.dat"));

const fn gradient_element(x: usize) -> u32 {
    let y = 0x80u8;
    let u = 0xc0u8;
    let v = ((x * 255 + 160) / 320) as u8;
    ((y as u32) << 24) | ((u as u32) << 16) | ((y as u32) << 8) | v as u32
}

const CHROMA_GRADIENT_ROW: [u32; WORDS_PER_SCANLINE] = array_const_fn_init![gradient_element; 320];

pub struct TextConsole {
    data: [[u8; Self::WIDTH]; Self::HEIGHT],
    modified: bool,
    origin_y: usize,
    cursor_x: usize,
    cursor_y: usize,
}

impl TextConsole {
    const FILL: u8 = b' ';
    const WIDTH: usize = SCREEN_WIDTH / CELL_WIDTH;
    const HEIGHT: usize = SCREEN_HEIGHT / CELL_HEIGHT - 2;

    pub const fn new() -> Self {
        Self {
            data: [[Self::FILL; Self::WIDTH]; Self::HEIGHT],
            modified: true,
            origin_y: 0,
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    pub fn modified(&self) -> bool {
        self.modified
    }

    pub fn render(&mut self, framebuffer: &Framebuffer) {
        self.modified = false;

        let mut dst = framebuffer.as_ptr();

        // SAFETY: render() writes exactly 300 KiB to a 300 KiB framebuffer.
        unsafe {
            Self::fill_gradient_row(&mut dst);
            let mut cell = self.origin_y;
            for _ in 0..Self::HEIGHT {
                for pixel in 0..CELL_HEIGHT {
                    self.fill_text_row(&mut dst, cell, pixel);
                }
                cell += 1;
                if cell >= Self::HEIGHT {
                    cell = 0;
                }
            }
            Self::fill_gradient_row(&mut dst);
        }
    }

    unsafe fn fill_gradient_row(dst: &mut *mut ()) {
        for _y in 0..CELL_HEIGHT {
            let mut src = CHROMA_GRADIENT_ROW.as_ptr();

            // Copy and flush 32-byte cache lines.
            for _cache_line_x in 0..WORDS_PER_SCANLINE / CACHE_LINE_WORDS {
                for i in 0..CACHE_LINE_WORDS as isize {
                    *dst.cast::<u32>().offset(i) = *src.offset(i);
                }

                flush_data_cache_block(*dst);
                src = src.offset(CACHE_LINE_WORDS as isize);
                *dst = dst.byte_offset(CACHE_LINE_BYTES as isize);
            }
        }
    }

    unsafe fn fill_text_row(&self, dst: &mut *mut (), cell_y: usize, pixel_y: usize) {
        // Render and flush pairs of characters into 32-byte cache lines.
        for cell_pair_x in 0..Self::WIDTH / 2 {
            for i in 0..2 {
                let character = self.data[cell_y][2 * cell_pair_x + i] as usize;
                let src = FONT.0[character][pixel_y].as_ptr();

                // Within a cache line, process pixel pairs at a time for 32-bit writes.
                for pixel_pair in 0..CELL_WIDTH as isize / 2 {
                    let luma_0 = *src.offset(2 * pixel_pair);
                    let luma_1 = *src.offset(2 * pixel_pair + 1);

                    *dst.cast::<u32>().offset(4 * i as isize + pixel_pair) =
                        (luma_0 as u32) << 24 | (luma_1 as u32) << 8 | 0x00800080;
                }
            }

            flush_data_cache_block(*dst);
            *dst = dst.byte_offset(CACHE_LINE_BYTES as isize);
        }
    }

    /// Shifts the view down one line, removing a line at the top and adding a blank line at the
    /// bottom.
    fn scroll_down(&mut self) {
        self.modified = true;

        let new_last_row = self.origin_y;

        self.origin_y += 1;
        if self.origin_y >= Self::HEIGHT {
            self.origin_y = 0;
        }

        for x in 0..Self::WIDTH {
            self.data[new_last_row][x] = Self::FILL;
        }
    }

    /// Moves the cursor one cell down, scrolling the view down to keep it visible.
    fn move_down(&mut self) {
        self.cursor_y += 1;
        if self.cursor_y >= Self::HEIGHT {
            self.cursor_y = 0;
        }

        if self.cursor_y == self.origin_y {
            // The cursor just wrapped off the bottom.
            self.scroll_down();
        }
    }

    /// Moves the cursor one cell right, wrapping to the first column of the next row.
    fn move_right(&mut self) {
        self.cursor_x += 1;
        if self.cursor_x >= Self::WIDTH {
            // The cursor just wrapped off the right
            self.cursor_x = 0;
            self.move_down();
        }
    }

    /// Prints a byte, regardless of its interpretation as a character.
    fn print_byte(&mut self, b: u8) {
        self.modified = true;

        self.data[self.cursor_y][self.cursor_x] = b;
        self.move_right();
    }

    /// Prints a character.
    ///
    /// ASCII control characters are interpreted. Non-ASCII characters are printed as a replacement
    /// character.
    fn print_char(&mut self, c: char) {
        match c {
            '\n' => {
                self.move_down();
                self.cursor_x = 0;
            }
            _ => {
                if c.is_ascii() {
                    if !c.is_ascii_control() {
                        self.print_byte(c as u8);
                    }
                } else {
                    self.print_byte(b'?');
                }
            }
        }
    }
}

impl Write for TextConsole {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.modified = true;

        for c in s.chars() {
            self.print_char(c);
        }
        Ok(())
    }
}
