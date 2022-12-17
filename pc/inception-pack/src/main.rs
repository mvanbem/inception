#[cfg(test)]
#[macro_use]
extern crate quickcheck_macros;

use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fs::{read, read_dir, File};
use std::hash::{Hash, Hasher};
use std::io::{stdout, Write};
use std::panic::resume_unwind;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread::spawn;

use anyhow::{anyhow, bail, Context, Result};
use clap::{clap_app, crate_authors, crate_description, crate_version, ArgMatches};
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

fn main() -> Result<()> {
    let matches = clap_app!(app =>
        (name: "inception-pack")
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@arg hl2_base: --("hl2-base") <PATH> "Path to a Half-Life 2 installation")
        (@subcommand pack_map =>
            (about: "Packs a single map for use on GC/Wii")
            (@arg MAP: "Map name or path to map file if ending with \".bsp\" (default: d1_trainstation_01)")
            (@arg dst: --dst [PATH] "Path to write packed outputs (default: .)")
        )
        (@subcommand pack_all_maps =>
            (about: "Packs maps for use on GC/Wii")
            (@arg dst: --dst [PATH] "Path to write packed outputs (default: .)")
        )
        (@subcommand cat_lump =>
            (about: "Dumps an arbitrary BSP lump to stdout")
            (@arg MAP: "Map name (example: d1_trainstation_01)")
            (@arg LUMP: "Lump index (example: 40)")
        )
        (@subcommand pack_model =>
            (about: "Packs a single model for use on GC/Wii")
            (@arg MODEL: "Model name (default: police)")
            (@arg dst: --dst [PATH] "Path to write packed outputs (default: .)")
        )
        (@subcommand cat_material =>
            (about: "Prints a material definition to stdout")
            (@arg NAME: ... "Material name (example: tile/tilefloor013a)")
        )
        (@subcommand describe_texture =>
            (about: "Prints texture metadata to stdout")
            (@arg NAME: ... "Texture name (example: tile/tilefloor013a)")
        )
        (@subcommand build_ui_font =>
            (about: "Builds the UI font to stdout")
        )
    )
    .get_matches();

    let hl2_base = Path::new(matches.value_of("hl2_base").unwrap());
    match matches.subcommand() {
        ("pack_map", Some(matches)) => {
            pack_map(hl2_base, matches.value_of("dst"), matches.value_of("MAP"))?
        }
        ("pack_all_maps", Some(matches)) => pack_all_maps(hl2_base, matches.value_of("dst"))?,
        ("cat_lump", Some(matches)) => cat_lump(hl2_base, matches)?,
        ("pack_model", Some(matches)) => {
            pack_model(hl2_base, matches.value_of("dst"), matches.value_of("MODEL"))?
        }
        ("cat_material", Some(matches)) => cat_material(hl2_base, matches)?,
        ("describe_texture", Some(matches)) => describe_texture(hl2_base, matches)?,
        ("build_ui_font", _) => build_ui_font()?,
        (name, _) => bail!("unknown subcommand: {:?}", name),
    }
    Ok(())
}

fn cat_lump(hl2_base: &Path, matches: &ArgMatches) -> Result<()> {
    let map_path = {
        let mut path = hl2_base.join("maps");
        path.push(format!("{}.bsp", matches.value_of("MAP").unwrap(),));
        path
    };
    let bsp_file =
        File::open(&map_path).with_context(|| format!("Opening map file {:?}", map_path))?;
    let bsp_data = unsafe { Mmap::map(&bsp_file) }?;
    let bsp = Bsp::new(&bsp_data);

    let lump_index = matches.value_of("LUMP").unwrap().parse().unwrap();
    let lump_data = bsp.lump_data(lump_index);

    let stdout = stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(lump_data)?;
    stdout.flush()?;

    Ok(())
}

fn cat_material(hl2_base: &Path, matches: &ArgMatches) -> Result<()> {
    let file_loader = Vpk::new(hl2_base.join("hl2_misc"))?;
    for name in matches.values_of("NAME").unwrap() {
        let path = VpkPath::new_with_prefix_and_extension(name, "materials", "vmt");
        let file = match file_loader.load_file(&path)? {
            Some(data) => data,
            None => bail!("asset not found: {}", path),
        };
        let stdout = stdout();
        let mut stdout = stdout.lock();
        stdout.write_all(&file)?;
        stdout.flush()?;
    }

    Ok(())
}

fn describe_texture(hl2_base: &Path, matches: &ArgMatches) -> Result<()> {
    let material_loader = Rc::new(Vpk::new(hl2_base.join("hl2_misc"))?);
    let texture_loader = Rc::new(Vpk::new(hl2_base.join("hl2_textures"))?);
    let asset_loader = AssetLoader::new(material_loader, texture_loader);
    for name in matches.values_of("NAME").unwrap() {
        let path = VpkPath::new_with_prefix_and_extension(name, "materials", "vtf");
        let vtf = asset_loader.get_texture(&path)?;
        println!("width: {}", vtf.width());
        println!("height: {}", vtf.height());
        println!("flags: 0x{:08x}", vtf.flags());
        println!("mips: {}", vtf.mips().len());
        println!("faces: {}", vtf.face_count());
        println!("format: {:?}", vtf.format());
    }

    Ok(())
}

fn pack_all_maps(hl2_base: &Path, dst: Option<&str>) -> Result<()> {
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
            let dst = dst.map(ToString::to_string);
            let map_queue = Arc::clone(&map_queue);
            move || -> Result<()> {
                loop {
                    let map_path = match map_queue.lock().unwrap().pop_front() {
                        Some(map_path) => map_path,
                        None => break,
                    };
                    println!("Pulled {} from the queue", map_path);
                    pack_map(&hl2_base, dst.as_ref().map(String::as_str), Some(&map_path))
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
