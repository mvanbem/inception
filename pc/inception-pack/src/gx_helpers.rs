use gx::bp::*;
use gx::display_list::*;
use inception_render_common::map_data::TextureTableEntry;

pub trait DisplayListExt {
    fn append_texcoord_scale(
        &mut self,
        texcoord: u8,
        texture_id: u16,
        texture_table: &[TextureTableEntry],
    );

    fn append_bind_texture(
        &mut self,
        image: u8,
        texture_id: u16,
        texture_table: &[TextureTableEntry],
    );
}

impl DisplayListExt for DisplayList {
    fn append_texcoord_scale(
        &mut self,
        texcoord: u8,
        texture_id: u16,
        texture_table: &[TextureTableEntry],
    ) {
        let entry = &texture_table[texture_id as usize];
        self.commands.push(Command::WriteBpReg {
            packed_addr_and_value: BpTexCoordRegA::new()
                .with_addr(BpTexCoordRegA::addr_for_texcoord(texcoord).unwrap())
                .with_s_scale_minus_one(entry.width - 1)
                .with_s_range_bias(false)
                .with_s_cylindrical_wrapping(false)
                .with_offset_for_lines(false)
                .with_offset_for_points(false)
                .into(),
            reference: None,
        });
        self.commands.push(Command::WriteBpReg {
            packed_addr_and_value: BpTexCoordRegB::new()
                .with_addr(BpTexCoordRegB::addr_for_texcoord(texcoord).unwrap())
                .with_t_scale_minus_one(entry.height - 1)
                .with_t_range_bias(false)
                .with_t_cylindrical_wrapping(false)
                .into(),
            reference: None,
        });
    }

    fn append_bind_texture(
        &mut self,
        image: u8,
        texture_id: u16,
        texture_table: &[TextureTableEntry],
    ) {
        let entry = &texture_table[texture_id as usize];
        self.commands.push(Command::WriteBpReg {
            packed_addr_and_value: BpTexModeRegA::new()
                .with_addr(BpTexModeRegA::addr_for_image(image).unwrap())
                .with_wrap_s(if entry.flags & TextureTableEntry::FLAG_CLAMP_S != 0 {
                    Wrap::Clamp
                } else {
                    Wrap::Repeat
                })
                .with_wrap_t(if entry.flags & TextureTableEntry::FLAG_CLAMP_T != 0 {
                    Wrap::Clamp
                } else {
                    Wrap::Repeat
                })
                .with_mag_filter(MagFilter::Linear)
                .with_min_filter(if entry.mip_count > 1 {
                    MinFilter::LinearMipLinear
                } else {
                    MinFilter::Linear
                })
                .with_diag_lod(DiagLod::EdgeLod)
                .with_lod_bias(0)
                .with_max_aniso(MaxAniso::_1)
                .with_lod_clamp(true)
                .into(),
            reference: None,
        });
        self.commands.push(Command::WriteBpReg {
            packed_addr_and_value: BpTexModeRegB::new()
                .with_addr(BpTexModeRegB::addr_for_image(image).unwrap())
                .with_min_lod(0)
                .with_max_lod((entry.mip_count - 1) << 4)
                .into(),
            reference: None,
        });
        self.commands.push(Command::WriteBpReg {
            packed_addr_and_value: BpTexImageRegA::new()
                .with_addr(BpTexImageRegA::addr_for_image(image).unwrap())
                .with_width_minus_one(entry.width - 1)
                .with_height_minus_one(entry.height - 1)
                .with_format(match entry.format {
                    1 => gx::bp::TextureFormat::I8,
                    3 => gx::bp::TextureFormat::Ia8,
                    6 => gx::bp::TextureFormat::Rgba8,
                    14 => gx::bp::TextureFormat::Cmp,
                    x => panic!("unexpected texture format {x}"),
                })
                .into(),
            reference: None,
        });
        self.commands.push(Command::WriteBpReg {
            packed_addr_and_value: BpTexImageRegB::new()
                .with_addr(BpTexImageRegB::addr_for_image(image).unwrap())
                .with_tmem_offset(((image as u32 * 128 * 1024) >> 5) as u16)
                .with_cache_width(CacheSize::_128KB)
                .with_cache_height(CacheSize::_128KB)
                .with_image_type(ImageType::Cached)
                .into(),
            reference: None,
        });
        self.commands.push(Command::WriteBpReg {
            packed_addr_and_value: BpTexImageRegC::new()
                .with_addr(BpTexImageRegC::addr_for_image(image).unwrap())
                .with_tmem_offset((((image as u32 * 128 + 512) * 1024) >> 5) as u16)
                .with_cache_width(CacheSize::_128KB)
                .with_cache_height(CacheSize::_128KB)
                .into(),
            reference: None,
        });
        self.commands
            .push(Command::write_bp_tex_image_reg_d_reference(image, texture_id).unwrap());
    }
}
