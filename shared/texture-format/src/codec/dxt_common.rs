use byteorder::{LittleEndian, ReadBytesExt};

fn rgb565_to_rgba8(rgb565: u16) -> [u8; 4] {
    let extend5 = |x| (x << 3) | (x >> 2);
    let extend6 = |x| (x << 2) | (x >> 4);
    [
        extend5(((rgb565 >> 11) & 0x1f) as u8),
        extend6(((rgb565 >> 5) & 0x3f) as u8),
        extend5((rgb565 & 0x1f) as u8),
        255,
    ]
}

pub fn blend(a: u8, b: u8, a_mul: u16, b_mul: u16, div: u16) -> u8 {
    ((a_mul * a as u16 + b_mul * b as u16) / div) as u8
}

fn blend_rgba(a: [u8; 4], b: [u8; 4], a_mul: u16, b_mul: u16, div: u16) -> [u8; 4] {
    [
        blend(a[0], b[0], a_mul, b_mul, div),
        blend(a[1], b[1], a_mul, b_mul, div),
        blend(a[2], b[2], a_mul, b_mul, div),
        blend(a[3], b[3], a_mul, b_mul, div),
    ]
}

fn block_color_a(block: &[u8; 8]) -> u16 {
    (&block[..2]).read_u16::<LittleEndian>().unwrap()
}

fn block_color_b(block: &[u8; 8]) -> u16 {
    (&block[2..4]).read_u16::<LittleEndian>().unwrap()
}

fn color_0(_color_a: u16, _color_b: u16, color_a_rgba: [u8; 4], _color_b_rgba: [u8; 4]) -> [u8; 4] {
    color_a_rgba
}

fn color_1(_color_a: u16, _color_b: u16, _color_a_rgba: [u8; 4], color_b_rgba: [u8; 4]) -> [u8; 4] {
    color_b_rgba
}

fn color_2(color_a: u16, color_b: u16, color_a_rgba: [u8; 4], color_b_rgba: [u8; 4]) -> [u8; 4] {
    if color_a > color_b {
        blend_rgba(color_a_rgba, color_b_rgba, 2, 1, 3)
    } else {
        blend_rgba(color_a_rgba, color_b_rgba, 1, 1, 2)
    }
}

fn color_3(color_a: u16, color_b: u16, color_a_rgba: [u8; 4], color_b_rgba: [u8; 4]) -> [u8; 4] {
    if color_a > color_b {
        blend_rgba(color_a_rgba, color_b_rgba, 1, 2, 3)
    } else {
        [0, 0, 0, 0]
    }
}

fn block_color_index(block: &[u8; 8], x: usize, y: usize) -> usize {
    let bit_index = 2 * (4 * y + x);
    let bits = (&block[4..]).read_u32::<LittleEndian>().unwrap();
    ((bits >> bit_index) & 3) as usize
}

pub fn block_color(block: &[u8; 8], x: usize, y: usize) -> [u8; 4] {
    let color_a = block_color_a(block);
    let color_b = block_color_b(block);
    let color_a_rgba = rgb565_to_rgba8(block_color_a(block));
    let color_b_rgba = rgb565_to_rgba8(block_color_b(block));
    match block_color_index(block, x, y) {
        0 => color_0(color_a, color_b, color_a_rgba, color_b_rgba),
        1 => color_1(color_a, color_b, color_a_rgba, color_b_rgba),
        2 => color_2(color_a, color_b, color_a_rgba, color_b_rgba),
        3 => color_3(color_a, color_b, color_a_rgba, color_b_rgba),
        _ => unreachable!(),
    }
}
