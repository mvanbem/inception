use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;

use anyhow::Result;

pub struct RgbU8Image {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

impl RgbU8Image {
    pub fn new(width: usize, height: usize, data: Vec<u8>) -> Self {
        assert_eq!(data.len(), 3 * width * height);
        Self {
            width,
            height,
            data,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn write_to_png(&self, path: &str) -> Result<()> {
        let w = BufWriter::new(File::create(path)?);
        let mut encoder = png::Encoder::new(w, self.width as u32, self.height as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&self.data)?;
        Ok(())
    }
}

#[derive(Default)]
pub struct TextureAtlas {
    patches: Vec<(usize, usize)>,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct PatchId(isize);

impl PatchId {
    fn new(index: usize, width: usize, height: usize) -> Self {
        if height > width {
            Self(-(index as isize) - 1)
        } else {
            Self(index as isize)
        }
    }

    pub fn is_flipped(self) -> bool {
        self.0 < 0
    }
}

impl TextureAtlas {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.patches.is_empty()
    }

    pub fn insert(&mut self, width: usize, height: usize) -> PatchId {
        let id = PatchId::new(self.patches.len(), width, height);
        self.patches.push((width, height));
        id
    }

    pub fn bake(self, width: usize, height: usize) -> Result<HashMap<PatchId, [usize; 2]>, Self> {
        let mut open = vec![(0, 0, width, height)];

        let mut offsets_by_patch_id = HashMap::new();
        let mut patches: Vec<(PatchId, (usize, usize))> = self
            .patches
            .iter()
            .enumerate()
            .map(|(index, &(patch_width, patch_height))| {
                (
                    PatchId::new(index, patch_width, patch_height),
                    (patch_width, patch_height),
                )
            })
            .collect();
        patches.sort_by_key(|&(_, (patch_width, patch_height))| patch_width * patch_height);
        'for_each_patch: while let Some((patch_id, (patch_width, patch_height))) = patches.pop() {
            let (oriented_patch_width, oriented_patch_height) = if patch_id.is_flipped() {
                (patch_height, patch_width)
            } else {
                (patch_width, patch_height)
            };
            if oriented_patch_width > width || oriented_patch_height > height {
                return Err(self);
            }

            // Consider smaller open spaces first.
            //open.sort_by_key(|&(_, _, width, height)| width * height);
            open.sort_by(|&(_, _, wa, ha), &(_, _, wb, hb)| wa.cmp(&wb).then_with(|| ha.cmp(&hb)));

            for (open_index, (open_x0, open_y0, open_width, open_height)) in
                open.iter().copied().enumerate()
            {
                if open_width < oriented_patch_width || open_height < oriented_patch_height {
                    continue;
                }

                // Found a sufficiently sized open space. Place this patch there.
                offsets_by_patch_id.insert(patch_id, [open_x0, open_y0]);

                // Remove the open space that was just used and add any leftover areas.
                open.remove(open_index);
                // Reserve entire S3TC/DXT1/BC1 blocks to keep lightmaps from popping horribly.
                let used_width = (oriented_patch_width + 3) & !3;
                let used_height = (oriented_patch_height + 3) & !3;
                if used_width < open_width {
                    // There is unused space to the right of the placed patch. Limit this open space
                    // to the patch's height, leaving the full width available for the next check.
                    open.push((
                        open_x0 + used_width,
                        open_y0,
                        open_width - used_width,
                        used_height,
                    ));
                }
                if used_height < open_height {
                    // There is unused space below the placed patch. Claim the entire width, which
                    // was left open just above.
                    open.push((
                        open_x0,
                        open_y0 + used_height,
                        open_width,
                        open_height - used_height,
                    ));
                }

                // Successfully placed this patch. Move on to the next patch.
                continue 'for_each_patch;
            }
            return Err(self);
        }

        Ok(offsets_by_patch_id)
    }

    pub fn bake_smallest(mut self) -> (usize, usize, HashMap<PatchId, [usize; 2]>) {
        let mut width = 1;
        let mut height = 1;
        loop {
            match self.bake(width, height) {
                Ok(result) => return (width, height, result),
                Err(recovered) => self = recovered,
            }

            if width == height {
                width *= 2;
            } else {
                height *= 2;
            }

            if width > 1024 {
                panic!("unable to bake atlas within the size limit");
            }
        }
    }
}
