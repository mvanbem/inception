use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};
use std::fs::{read_dir, File};
use std::io::{self, BufReader, BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use byteorder::{BigEndian, WriteBytesExt};
use chrono::{DateTime, Datelike};
use clap::StructOpt;
use relocation::{PointerFormat, RelocationWriter};

#[derive(clap::Parser)]
#[clap(name = "build-gcm")]
struct Args {
    #[clap(long)]
    apploader: String,

    #[clap(long)]
    dol: String,

    #[clap(long)]
    root_directory: String,

    #[clap(long)]
    output: String,
}

// fn zero_pad<const N: usize>(bytes: &[u8]) -> [u8; N] {
//     let mut result = [0; N];
//     result[..bytes.len()].copy_from_slice(bytes);
//     result
// }

trait PadToAlignedExt: Seek + Write {
    fn zero_pad_to_alignment<const N: usize>(&mut self) -> io::Result<()> {
        assert_eq!(N.count_ones(), 1);

        let position = self.stream_position()?;
        let goal_position = (position + N as u64 - 1) & !(N as u64 - 1);
        let padding_len = (goal_position - position) as usize;
        let padding = [0; N];
        self.write_all(&padding[..padding_len])
    }
}

impl<W: Seek + Write> PadToAlignedExt for W {}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut output = RelocationWriter::new(BufWriter::new(File::create(&args.output)?));

    write_disk_header(&mut output)?;
    write_apploader(&mut output, &args)?;
    write_dol(&mut output, &args)?;
    write_fst(&mut output, &args)?;
    output.zero_pad_to_alignment::<2048>()?;

    output.finish()?.flush()?;
    Ok(())
}

fn write_disk_header(output: &mut RelocationWriter<impl Seek + Write>) -> Result<()> {
    output.write_all(b"GGME")?; // Game code (Gamecube, "GM", US)
    output.write_all(b"MV")?; // Maker code
    output.write_u8(0)?; // Disc ID
    output.write_u8(0)?; // Version
    output.write_u8(0)?; // Audio streaming (no)
    output.write_u8(0)?; // Stream buffer size (default)
    output.seek(SeekFrom::Start(0x1c))?;
    output.write_u32::<BigEndian>(0xc2339f3d)?; // GameCube magic word.
    output.write_all(b"Inception")?;
    output.seek(SeekFrom::Start(0x420))?;
    output.write_pointer(PointerFormat::BigEndianU32, Cow::Borrowed("dol"))?;
    output.write_pointer(PointerFormat::BigEndianU32, Cow::Borrowed("fst"))?;
    output.write_pointer(PointerFormat::BigEndianU32, Cow::Borrowed("fst_size"))?;
    output.write_pointer(PointerFormat::BigEndianU32, Cow::Borrowed("fst_size"))?;
    Ok(())
}

fn write_apploader(output: &mut RelocationWriter<impl Seek + Write>, args: &Args) -> Result<()> {
    // Write the apploader's modified date.
    let apploader_file = File::open(&args.apploader)?;
    let modified: DateTime<chrono::Local> = apploader_file.metadata()?.modified()?.into();
    output.seek(SeekFrom::Start(0x2440))?;
    output.write_all(
        format!(
            "{:04}/{:02}/{:02}",
            modified.year(),
            modified.month(),
            modified.day()
        )
        .as_bytes(),
    )?;

    // Write the rest of the header.
    output.seek(SeekFrom::Start(0x2450))?;
    output.write_u32::<BigEndian>(0x81200000)?; // Apploader entry point
    output.write_pointer(PointerFormat::BigEndianU32, Cow::Borrowed("apploader_size"))?;
    output.write_u32::<BigEndian>(0)?; // Trailer size

    // Write the apploader.
    output.seek(SeekFrom::Start(0x2460))?;
    io::copy(&mut BufReader::new(apploader_file), &mut **output)?;
    output.zero_pad_to_alignment::<32>()?;
    let apploader_size = output.stream_position()? - 0x2460;
    output.define_symbol(Cow::Borrowed("apploader_size"), apploader_size);

    Ok(())
}

fn write_dol(output: &mut RelocationWriter<impl Seek + Write>, args: &Args) -> Result<()> {
    output.zero_pad_to_alignment::<2048>()?;
    output.define_symbol_here(Cow::Borrowed("dol"))?;
    io::copy(&mut BufReader::new(File::open(&args.dol)?), &mut **output)?;

    Ok(())
}

enum FlattenedFstEntry<'a> {
    File {
        name: &'a str,
        file: &'a FstFile,
    },
    Directory {
        name: &'a str,
        parent_offset: u32,
        next_offset: u32,
    },
}

enum FstEntry {
    File(FstFile),
    Directory(FstDirectory),
}

struct FstFile {
    path: PathBuf,
    len: u64,
}

struct FstDirectory {
    entries: BTreeMap<String, FstEntry>,
}

impl FstDirectory {
    fn new(path: &Path) -> Result<Self> {
        let mut entries = BTreeMap::default();
        for entry in read_dir(path)? {
            let entry = entry?;
            let name = entry.file_name().into_string().unwrap();
            if entry.file_type()?.is_dir() {
                entries.insert(name, FstEntry::Directory(Self::new(&entry.path())?));
            } else {
                entries.insert(
                    name,
                    FstEntry::File(FstFile {
                        path: entry.path(),
                        len: entry.metadata()?.len(),
                    }),
                );
            }
        }
        Ok(Self { entries })
    }

    fn flatten_to<'a>(
        &'a self,
        name: &'a str,
        parent_offset: u32,
        entries: &mut Vec<FlattenedFstEntry<'a>>,
    ) {
        // Add a placeholder entry for this directory.
        entries.push(FlattenedFstEntry::Directory {
            name,
            parent_offset,
            next_offset: u32::MAX, // Placeholder
        });
        let self_entry_index = entries.len() - 1;
        let offset = u32::try_from(12 * self_entry_index).unwrap();

        // Recursively traverse the entries.
        for (name, entry) in &self.entries {
            match entry {
                FstEntry::File(file) => entries.push(FlattenedFstEntry::File { name, file }),
                FstEntry::Directory(directory) => {
                    directory.flatten_to(name, offset, entries);
                }
            }
        }

        // Fill in the placeholder now that we know how many entries the subtree spans.
        let entries_len = entries.len();
        match &mut entries[self_entry_index] {
            FlattenedFstEntry::Directory { next_offset, .. } => {
                *next_offset = u32::try_from(12 * entries_len).unwrap()
            }
            _ => unreachable!(),
        }
    }

    fn write_strings<'a>(
        &'a self,
        string_table: &mut StringTableWriter<'a, impl Seek + Write>,
    ) -> Result<()> {
        for (name, entry) in &self.entries {
            string_table.maybe_write_string(name)?;
            if let FstEntry::Directory(directory) = entry {
                directory.write_strings(string_table)?;
            }
        }
        Ok(())
    }

    fn write_files(&self, output: &mut RelocationWriter<impl Seek + Write>) -> Result<()> {
        for entry in self.entries.values() {
            match entry {
                FstEntry::File(file) => {
                    output.zero_pad_to_alignment::<2048>()?;
                    let file_start = output.stream_position()?;
                    output.define_symbol(file_data_symbol(&file.path), file_start);
                    io::copy(&mut BufReader::new(File::open(&file.path)?), &mut **output)?;
                    let file_len = output.stream_position()? - file_start;
                    assert_eq!(file_len, file.len);
                }
                FstEntry::Directory(directory) => directory.write_files(output)?,
            }
        }
        Ok(())
    }
}

struct StringTableWriter<'a, W> {
    w: &'a mut RelocationWriter<W>,
    base: u64,
    strings_written: HashSet<&'a str>,
}

impl<'a, W: Seek + Write> StringTableWriter<'a, W> {
    fn new(w: &'a mut RelocationWriter<W>) -> Result<Self> {
        let base = w.stream_position()?;
        Ok(Self {
            w,
            base,
            strings_written: Default::default(),
        })
    }

    fn maybe_write_string(&mut self, s: &'a str) -> Result<()> {
        if self.strings_written.insert(s) {
            let pos = self.w.stream_position()?;
            self.w
                .define_symbol(string_table_symbol(s), pos - self.base);
            self.w.write_all(s.as_bytes())?;
            self.w.write_u8(0)?;
        }
        Ok(())
    }
}

fn string_table_symbol(s: &str) -> Cow<'static, str> {
    Cow::Owned(format!("str:{}", s))
}

fn file_data_symbol(path: &Path) -> Cow<'static, str> {
    Cow::Owned(format!("file_data:{}", path.display()))
}

fn write_fst(output: &mut RelocationWriter<impl Seek + Write>, args: &Args) -> Result<()> {
    // Scan the root directory, build a metadata tree, and flatten it.
    let root_directory = FstDirectory::new(Path::new(&args.root_directory))?;
    let mut entries = Vec::new();
    root_directory.flatten_to("<root>", 0, &mut entries);

    // Write a FST entry for each file and directory.
    output.zero_pad_to_alignment::<2048>()?;
    let fst_start = output.stream_position()?;
    output.define_symbol(Cow::Borrowed("fst"), fst_start);
    for (index, entry) in entries.into_iter().enumerate() {
        match entry {
            FlattenedFstEntry::File { name, file } => {
                output.write_u8(0)?; // Type: file
                output.write_pointer(PointerFormat::BigEndianU24, string_table_symbol(name))?;
                output.write_pointer(PointerFormat::BigEndianU32, file_data_symbol(&file.path))?;
                output.write_u32::<BigEndian>(u32::try_from(file.len).unwrap())?;
            }
            FlattenedFstEntry::Directory {
                name,
                parent_offset,
                next_offset,
            } => {
                output.write_u8(1)?; // Type: directory
                output.write_pointer(PointerFormat::BigEndianU24, string_table_symbol(name))?;
                output.write_u32::<BigEndian>(parent_offset)?;
                output.write_u32::<BigEndian>(if index == 0 {
                    next_offset / 12
                } else {
                    next_offset
                })?;
            }
        }
    }

    // Write the string table.
    let mut string_table = StringTableWriter::new(output)?;
    string_table.maybe_write_string("<root>")?;
    root_directory.write_strings(&mut string_table)?;

    let fst_size = output.stream_position()? - fst_start;
    output.define_symbol(Cow::Borrowed("fst_size"), fst_size);

    // Write file contents.
    root_directory.write_files(output)?;

    Ok(())
}
