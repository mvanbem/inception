use core::mem::zeroed;
use core::ops::Deref;

use inception_render_common::map_data::MapData;
use ogc_sys::*;

use crate::memalign::Memalign;

pub struct Lightmap {
    image_data: Memalign<32>,
    texobj: GXTexObj,
}

impl Lightmap {
    pub fn new<Data: Deref<Target = [u8]>>(map_data: &MapData<Data>, cluster_index: usize) -> Self {
        let cluster = &map_data.lightmap_cluster_table()[cluster_index];
        let coarse_width = ((cluster.width + 3) / 4).max(1);
        let coarse_height = ((cluster.height + 3) / 4).max(1);
        let physical_width = 4 * coarse_width;
        let physical_height = 4 * coarse_height;

        let image_data =
            Memalign::<32>::new(4 * physical_width as usize * physical_height as usize);
        unsafe { libc::memset(image_data.as_void_ptr_mut(), 0, image_data.size()) };
        unsafe { image_data.dc_flush() };

        let mut texobj = unsafe { zeroed::<GXTexObj>() };
        unsafe {
            GX_InitTexObj(
                &mut texobj,
                image_data.as_void_ptr_mut(),
                physical_width as u16,
                physical_height as u16,
                GX_TF_CMPR as u8,
                GX_CLAMP as u8,
                GX_CLAMP as u8,
                GX_FALSE as u8,
            );
            GX_InitTexObjFilterMode(&mut texobj, GX_NEAR as u8, GX_LINEAR as u8);
        }

        Self {
            image_data: image_data,
            texobj,
        }
    }

    pub fn update<Data: Deref<Target = [u8]>>(&mut self, map_data: &MapData<Data>, cluster_index: usize, style: usize) {
        assert!(style < 4);

        let cluster = &map_data.lightmap_cluster_table()[cluster_index];
        let blocks_wide = ((cluster.width + 7) / 8).max(1) as usize;

        let patches = &map_data.lightmap_patch_table()
            [cluster.patch_table_start_index as usize..cluster.patch_table_end_index as usize];
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

                    self.image_data.as_mut()[dst_offset..dst_offset + 8]
                        .copy_from_slice(&page_data[src_offset..src_offset + 8]);
                }
            }
        }
        unsafe {
            self.image_data.dc_flush();
            GX_InvalidateTexAll();
        }
    }

    pub fn texobj(&self) -> *mut GXTexObj {
        &self.texobj as *const GXTexObj as *mut GXTexObj
    }
}
