#[cfg(test)]
#[macro_use]
extern crate quickcheck_macros;

use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fs::{read, read_dir, File};
use std::hash::{Hash, Hasher};
use std::io::{stdout, Write};
use std::panic::resume_unwind;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread::spawn;

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use fontdue::{Font, FontSettings};
use memmap::Mmap;
use source_reader::asset::AssetLoader;
use source_reader::bsp::Bsp;
use source_reader::file::FileLoader;
use source_reader::vpk::path::VpkPath;
use source_reader::vpk::Vpk;
use texture_format::{TextureBuf, TextureFormat};

#[cfg(test)]
use quickcheck::Arbitrary;

use crate::map::pack_map;
use crate::model::pack_model;

mod counter;
mod draw_builder;
mod gx_helpers;
mod legacy_pass_params;
mod map;
mod model;
mod packed_material;
mod texture_key;
mod write_big_endian;

#[derive(Parser)]
struct Args {
    /// Path to a Half-Life 2 installation.
    #[clap(long)]
    hl2_base: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Packs a single map for use on GC/Wii.
    PackMap {
        /// Map name or path to map file if ending with ".bsp" (example: d1_trainstation_01)
        map: String,
        /// Path to write packed outputs
        #[arg(long, default_value = ".")]
        dst: PathBuf,
    },
    /// Packs maps for use on GC/Wii.
    PackAllMaps {
        /// Path to write packed outputs
        #[arg(default_value = ".")]
        dst: PathBuf,
    },
    /// Dumps an arbitrary BSP lump to stdout.
    CatLump {
        /// Map name (example: d1_trainstation_01)
        map_name: String,
        /// Lump index (example: 40)
        lump_index: usize,
    },
    /// Packs a single model for use on GC/Wii.
    PackModel {
        /// Model name (example: police)
        name: String,
        /// Path to write packed outputs
        #[arg(long, default_value = ".")]
        dst: PathBuf,
    },
    /// Prints a material definition to stdout.
    CatMaterial {
        /// Material name (example: tile/tilefloor013a)
        name: String,
    },
    /// Prints texture metadata to stdout.
    DescribeTexture {
        /// Texture name (example: tile/tilefloor013a)
        name: String,
    },
    /// Builds the UI font to stdout.
    BuildUiFont,
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::PackMap { map, dst } => pack_map(&args.hl2_base, &dst, &map)?,
        Command::PackAllMaps { dst } => pack_all_maps(&args.hl2_base, &dst)?,
        Command::CatLump {
            map_name,
            lump_index,
        } => cat_lump(&args.hl2_base, &map_name, lump_index)?,
        Command::PackModel { name, dst } => pack_model(&args.hl2_base, &dst, &name)?,
        Command::CatMaterial { name } => cat_material(&args.hl2_base, &name)?,
        Command::DescribeTexture { name } => describe_texture(&args.hl2_base, &name)?,
        Command::BuildUiFont => build_ui_font()?,
    }
    Ok(())
}

fn cat_lump(hl2_base: &Path, map_name: &str, lump_index: usize) -> Result<()> {
    let map_path = {
        let mut path = hl2_base.join("maps");
        path.push(format!("{}.bsp", map_name));
        path
    };
    let bsp_file =
        File::open(&map_path).with_context(|| format!("Opening map file {:?}", map_path))?;
    let bsp_data = unsafe { Mmap::map(&bsp_file) }?;
    let bsp = Bsp::new(&bsp_data);

    let lump_data = bsp.lump_data(lump_index);

    let stdout = stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(lump_data)?;
    stdout.flush()?;

    Ok(())
}

fn cat_material(hl2_base: &Path, name: &str) -> Result<()> {
    let file_loader = Vpk::new(hl2_base.join("hl2_misc"))?;

    let path = VpkPath::new_with_prefix_and_extension(name, "materials", "vmt");
    let file = match file_loader.load_file(&path)? {
        Some(data) => data,
        None => bail!("asset not found: {}", path),
    };

    let stdout = stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(&file)?;
    stdout.flush()?;

    Ok(())
}

fn describe_texture(hl2_base: &Path, name: &str) -> Result<()> {
    let material_loader = Rc::new(Vpk::new(hl2_base.join("hl2_misc"))?);
    let texture_loader = Rc::new(Vpk::new(hl2_base.join("hl2_textures"))?);
    let asset_loader = AssetLoader::new(material_loader, texture_loader);

    let path = VpkPath::new_with_prefix_and_extension(name, "materials", "vtf");
    let vtf = asset_loader.get_texture(&path)?;

    println!("width: {}", vtf.width());
    println!("height: {}", vtf.height());
    println!("flags: 0x{:08x}", vtf.flags());
    println!("mips: {}", vtf.mips().len());
    println!("faces: {}", vtf.face_count());
    println!("format: {:?}", vtf.format());

    Ok(())
}

fn pack_all_maps(hl2_base: &Path, dst: &Path) -> Result<()> {
    let map_queue = Arc::new(Mutex::new(VecDeque::new()));
    let mut locked_queue = map_queue.lock().unwrap();
    for entry in read_dir(&hl2_base.join("maps"))? {
        let entry = entry?;
        if let Some(file_name) = entry.file_name().to_str() {
            if file_name.ends_with(".bsp")
                && entry.metadata()?.len() > 0
                && !file_name.ends_with("intro.bsp")
                && !file_name.ends_with("credits.bsp")
            {
                locked_queue.push_back(entry.path().to_str().unwrap().to_string());
            }
        }
    }
    drop(locked_queue);

    let mut threads = Vec::new();
    for _ in 0..8 {
        threads.push(spawn({
            let hl2_base = hl2_base.to_path_buf();
            let dst = PathBuf::from(dst);
            let map_queue = Arc::clone(&map_queue);
            move || -> Result<()> {
                loop {
                    let map_path = match map_queue.lock().unwrap().pop_front() {
                        Some(map_path) => map_path,
                        None => break,
                    };
                    println!("Pulled {} from the queue", map_path);
                    pack_map(&hl2_base, &dst, &map_path)
                        .with_context(|| format!("Packing map {}", map_path))?;
                }
                Ok(())
            }
        }));
    }
    for thread in threads {
        match thread.join() {
            Ok(result) => result?,
            Err(panic_payload) => resume_unwind(panic_payload),
        }
    }

    Ok(())
}

fn build_ui_font() -> Result<()> {
    const SCALE: f32 = 15.0;

    // Clamped lower bound to keep characters like underscore in the box.
    const LOWEST_YMIN: i32 = -2;

    // How far the baseline is raised from the bottom edge of the cell.
    const BASELINE_OFFSET: i32 = 2;

    let font_bytes = read("../third_party/dejavu-fonts-ttf-2.37/DejaVuSansMono.ttf")?;
    let font = Font::from_bytes(
        font_bytes,
        FontSettings {
            scale: SCALE,
            ..FontSettings::default()
        },
    )
    .map_err(|e| anyhow!(e))?;

    let mut texels = vec![0; 3 * 256 * 256];

    for c in 0x20 as char..=0x7f as char {
        let (metrics, coverage) = font.rasterize(c, SCALE);
        let x0 = ((c as i32) & 0xf) * 16 + metrics.xmin + metrics.width as i32 / 2;
        let y0 = ((c as i32) >> 4) * 16 + 16
            - metrics.height as i32
            - metrics.ymin.max(LOWEST_YMIN)
            - BASELINE_OFFSET;
        for dy in 0..metrics.height {
            for dx in 0..metrics.width {
                let x = x0 + dx as i32;
                let y = y0 + dy as i32;
                if x >= 0 && x < 256 && y >= 0 && y < 256 {
                    let src = metrics.width * dy + dx;
                    let dst = (3 * (256 * y + x)) as usize;
                    texels[dst] = coverage[src];
                    texels[dst + 1] = coverage[src];
                    texels[dst + 2] = coverage[src];
                }
            }
        }
    }

    let texture = TextureBuf::transcode(
        TextureBuf::new(TextureFormat::Rgb8, 256, 256, texels).as_slice(),
        TextureFormat::GxTfI8,
    );
    let stdout = stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(texture.data())?;
    stdout.flush()?;

    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub struct FloatByBits(f32);

#[cfg(test)]
impl Arbitrary for FloatByBits {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Self(f32::arbitrary(g))
    }
}

impl Eq for FloatByBits {}

impl Hash for FloatByBits {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialEq for FloatByBits {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits().eq(&other.0.to_bits())
    }
}

impl Ord for FloatByBits {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.to_bits().cmp(&other.0.to_bits())
    }
}

impl PartialOrd for FloatByBits {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.to_bits().partial_cmp(&other.0.to_bits())
    }
}

fn hashable_float<const N: usize>(array: &[f32; N]) -> [FloatByBits; N] {
    let mut result = [FloatByBits(0.0); N];
    for index in 0..N {
        result[index] = FloatByBits(array[index]);
    }
    result
}
