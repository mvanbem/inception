use stb_dxt::{stb_compress_dxt_block, STB_DXT_NORMAL};

use crate::codec::{dxt_common, Codec};
use crate::texture_format::BlockMetrics;
use crate::TextureFormat;

pub fn block_alpha_a(block: &[u8; 8]) -> u8 {
    block[0]
}

pub fn block_alpha_b(block: &[u8; 8]) -> u8 {
    block[1]
}

pub fn alpha_0(alpha_a: u8, _alpha_b: u8) -> u8 {
    alpha_a
}

pub fn alpha_1(_alpha_a: u8, alpha_b: u8) -> u8 {
    alpha_b
}

pub fn alpha_2(alpha_a: u8, alpha_b: u8) -> u8 {
    if alpha_a > alpha_b {
        dxt_common::blend(alpha_a, alpha_b, 6, 1, 7)
    } else {
        dxt_common::blend(alpha_a, alpha_b, 4, 1, 5)
    }
}

pub fn alpha_3(alpha_a: u8, alpha_b: u8) -> u8 {
    if alpha_a > alpha_b {
        dxt_common::blend(alpha_a, alpha_b, 5, 2, 7)
    } else {
        dxt_common::blend(alpha_a, alpha_b, 3, 2, 5)
    }
}

pub fn alpha_4(alpha_a: u8, alpha_b: u8) -> u8 {
    if alpha_a > alpha_b {
        dxt_common::blend(alpha_a, alpha_b, 4, 3, 7)
    } else {
        dxt_common::blend(alpha_a, alpha_b, 2, 3, 5)
    }
}

pub fn alpha_5(alpha_a: u8, alpha_b: u8) -> u8 {
    if alpha_a > alpha_b {
        dxt_common::blend(alpha_a, alpha_b, 3, 4, 7)
    } else {
        dxt_common::blend(alpha_a, alpha_b, 1, 4, 5)
    }
}

pub fn alpha_6(alpha_a: u8, alpha_b: u8) -> u8 {
    if alpha_a > alpha_b {
        dxt_common::blend(alpha_a, alpha_b, 2, 5, 7)
    } else {
        0
    }
}

pub fn alpha_7(alpha_a: u8, alpha_b: u8) -> u8 {
    if alpha_a > alpha_b {
        dxt_common::blend(alpha_a, alpha_b, 1, 6, 7)
    } else {
        255
    }
}

pub fn block_alpha_index(block: &[u8; 8], x: usize, y: usize) -> usize {
    let bit_index = 3 * (4 * y + x);
    let bits = block[2] as u64
        | ((block[3] as u64) << 8)
        | ((block[4] as u64) << 16)
        | ((block[5] as u64) << 24)
        | ((block[6] as u64) << 32)
        | ((block[7] as u64) << 40);
    ((bits >> bit_index) & 7) as usize
}

pub fn block_alpha(block: &[u8; 8], x: usize, y: usize) -> u8 {
    let alpha_a = block_alpha_a(block);
    let alpha_b = block_alpha_b(block);
    match block_alpha_index(block, x, y) {
        0 => alpha_0(alpha_a, alpha_b),
        1 => alpha_1(alpha_a, alpha_b),
        2 => alpha_2(alpha_a, alpha_b),
        3 => alpha_3(alpha_a, alpha_b),
        4 => alpha_4(alpha_a, alpha_b),
        5 => alpha_5(alpha_a, alpha_b),
        6 => alpha_6(alpha_a, alpha_b),
        7 => alpha_7(alpha_a, alpha_b),
        _ => unreachable!(),
    }
}

#[derive(Debug)]
pub struct Dxt5;

impl Codec for Dxt5 {
    const FORMAT: TextureFormat = TextureFormat::Dxt5;
    const METRICS: BlockMetrics = BlockMetrics {
        block_width: 4,
        block_height: 4,
        encoded_block_size: 16,
    };
    type EncodedBlock = [u8; 16];

    fn encode_block(texels: &[u8]) -> [u8; 16] {
        assert_eq!(texels.len(), 64);

        let mut compressed = [0; 16];
        unsafe {
            stb_compress_dxt_block(compressed.as_mut_ptr(), texels.as_ptr(), 1, STB_DXT_NORMAL);
        }
        compressed
    }

    fn get_texel(
        physical_width: usize,
        _physical_height: usize,
        data: &[u8],
        x: usize,
        y: usize,
    ) -> [u8; 4] {
        assert_eq!(physical_width % 4, 0);
        let blocks_wide = physical_width / 4;

        let coarse_x = x / 4;
        let coarse_y = y / 4;
        let fine_x = x % 4;
        let fine_y = y % 4;

        let offset = 16 * (blocks_wide * coarse_y + coarse_x);
        let alpha_block: &[u8; 8] = data[offset..offset + 8].try_into().unwrap();
        let color_block: &[u8; 8] = data[offset + 8..offset + 16].try_into().unwrap();

        let [r, g, b, _] = dxt_common::block_color(color_block, fine_x, fine_y);
        let a = block_alpha(alpha_block, fine_x, fine_y);

        [r, g, b, a]
    }
}
