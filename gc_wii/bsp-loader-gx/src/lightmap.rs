use core::mem::zeroed;
use core::ops::Deref;

use alloc::vec::Vec;
use inception_render_common::map_data::{CommonLightmapTableEntry, MapData};
use ogc_sys::*;

pub struct Lightmap {
    image_data: Vec<u8, GlobalAlign32>,
    texobj: GXTexObj,
}

impl Lightmap {
    pub fn new(entry: &CommonLightmapTableEntry) -> Self {
        let coarse_width = ((entry.width + 3) / 4).max(1);
        let coarse_height = ((entry.height + 3) / 4).max(1);
        let physical_width = 4 * coarse_width;
        let physical_height = 4 * coarse_height;

        let mut image_data = Vec::with_capacity_in(
            4 * physical_width as usize * physical_height as usize,
            GlobalAlign32,
        );
        unsafe {
            core::ptr::write_bytes(
                image_data.spare_capacity_mut().as_mut_ptr(),
                0,
                image_data.spare_capacity_mut().len(),
            );
            image_data.set_len(image_data.capacity());
            DCFlushRange(image_data.as_ptr() as _, image_data.len() as u32);
        };

        let mut texobj = unsafe { zeroed::<GXTexObj>() };
        unsafe {
            GX_InitTexObj(
                &mut texobj,
                image_data.as_ptr() as _,
                physical_width as u16,
                physical_height as u16,
                GX_TF_CMPR as u8,
                GX_CLAMP as u8,
                GX_CLAMP as u8,
                GX_FALSE as u8,
            );
            GX_InitTexObjFilterMode(&mut texobj, GX_NEAR as u8, GX_LINEAR as u8);
        }

        Self { image_data, texobj }
    }

    pub fn update<Data: Deref<Target = [u8]>>(
        &mut self,
        map_data: &MapData<Data>,
        entry: &CommonLightmapTableEntry,
        style: usize,
    ) {
        assert!(style < 4);

        let blocks_wide = ((entry.width + 7) / 8).max(1) as usize;

        let patches = &map_data.lightmap_patch_table()
            [entry.patch_table_start_index as usize..entry.patch_table_end_index as usize];
        for patch in patches {
            let style = style.min(patch.style_count as usize - 1);

            let patch_data = &map_data.lightmap_data()
                [patch.data_start_offset as usize..patch.data_end_offset as usize];
            let page_size = 8 * patch.sub_blocks_wide as usize * patch.sub_blocks_high as usize;
            let page_index = style;
            let page_offset = page_size * page_index;
            let page_data = &patch_data[page_offset..page_offset + page_size];

            for sub_block_dx in 0..patch.sub_blocks_wide {
                for sub_block_dy in 0..patch.sub_blocks_high {
                    let src_offset = 8
                        * (patch.sub_blocks_wide as usize * sub_block_dy as usize
                            + sub_block_dx as usize);
                    let dst_x = patch.sub_block_x as usize + sub_block_dx as usize;
                    let dst_y = patch.sub_block_y as usize + sub_block_dy as usize;
                    // bits: y..y x..x y x 000
                    //       \__/ \__/ | | \_/
                    //         |    |  | |  `-- byte within sub-block
                    //         |    |  |  `---- sub-block x position within block
                    //         |    |  `------- sub-block y position within block
                    //         |    `---------- block x position (as many as needed for width/8)
                    //         `--------------- block y position (as many as needed for height/8)
                    let dst_offset = 32 * (blocks_wide * (dst_y >> 1) + (dst_x >> 1))
                        + 16 * (dst_y & 1)
                        + 8 * (dst_x & 1);

                    self.image_data[dst_offset..dst_offset + 8]
                        .copy_from_slice(&page_data[src_offset..src_offset + 8]);
                }
            }
        }
        unsafe {
            DCFlushRange(self.image_data.as_ptr() as _, self.image_data.len() as u32);
            GX_InvalidateTexAll();
        }
    }

    pub fn texobj(&self) -> *mut GXTexObj {
        &self.texobj as *const GXTexObj as *mut GXTexObj
    }
}
