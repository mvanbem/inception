use core::ops::Index;

pub fn draw(display_list_start_offset: u32, display_list_end_offset: u32) -> [u32; 2] {
    assert_eq!(display_list_start_offset >> 24, 0);
    [display_list_start_offset, display_list_end_offset]
}

pub fn set_plane(texture_matrix: impl Index<(usize, usize), Output = f32>) -> [u32; 13] {
    [
        0x01000000,
        texture_matrix[(0, 0)].to_bits(),
        texture_matrix[(0, 1)].to_bits(),
        texture_matrix[(0, 2)].to_bits(),
        texture_matrix[(0, 3)].to_bits(),
        texture_matrix[(1, 0)].to_bits(),
        texture_matrix[(1, 1)].to_bits(),
        texture_matrix[(1, 2)].to_bits(),
        texture_matrix[(1, 3)].to_bits(),
        texture_matrix[(2, 0)].to_bits(),
        texture_matrix[(2, 1)].to_bits(),
        texture_matrix[(2, 2)].to_bits(),
        texture_matrix[(2, 3)].to_bits(),
    ]
}

pub fn set_base_texture(base_texture_index: u16) -> [u32; 1] {
    [0x02000000 | base_texture_index as u32]
}

pub fn set_env_map_texture(env_map_texture_index: u16) -> [u32; 1] {
    [0x03000000 | env_map_texture_index as u32]
}

pub fn set_env_map_tint(env_map_tint: [u8; 3]) -> [u32; 1] {
    [0x04000000
        | ((env_map_tint[0] as u32) << 16)
        | ((env_map_tint[1] as u32) << 8)
        | env_map_tint[2] as u32]
}

pub fn set_alpha(test: u8, threshold: u8, blend: u8) -> [u32; 1] {
    [0x05000000 | ((test as u32) << 16) | ((threshold as u32) << 8) | blend as u32]
}

pub fn set_aux_texture(aux_texture_index: u16) -> [u32; 1] {
    [0x06000000 | aux_texture_index as u32]
}
