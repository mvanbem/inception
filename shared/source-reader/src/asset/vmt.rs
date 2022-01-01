use std::rc::Rc;
use std::str::from_utf8;

use anyhow::{bail, Context, Result};
use nalgebra_glm::Vec3;

use crate::asset::{Asset, AssetLoader};
use crate::properties::{material_vector, Entry, KeyValue, Object};
use crate::vpk::path::VpkPath;

fn parse_bool(s: &str) -> Result<bool> {
    match s {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => match s.parse() {
            Ok(value) => Ok(value),
            Err(_) => bail!("bad bool parameter: {}", s),
        },
    }
}

fn parse_i32(s: &str) -> Result<i32> {
    match s.parse() {
        Ok(value) => Ok(value),
        Err(_) => bail!("bad i32 parameter: {}", s),
    }
}

fn parse_f32(s: &str) -> Result<f32> {
    match s.parse() {
        Ok(value) => Ok(value),
        Err(_) => bail!("bad f32 parameter: {}", s),
    }
}

fn parse_material_vector(s: &str) -> Result<Vec3> {
    match material_vector(s) {
        Ok(v) => Ok(v),
        Err(e) => bail!("{}", e),
    }
}

fn parse_vector_or_f32(s: &str) -> Result<f32> {
    // TODO: Expose the full vector. This is a quick hack.
    match parse_material_vector(s) {
        Ok(v) => Ok((v[0] + v[1] + v[2]) / 3.0),
        Err(_) => parse_f32(s),
    }
}

fn parse_vtf_path(s: &str) -> Result<Option<VpkPath>> {
    match s {
        "env_cubemap" => Ok(None),
        _ => Ok(Some(VpkPath::new_with_prefix_and_extension(
            s,
            "materials",
            "vtf",
        ))),
    }
}

fn parse_vmt_path(s: &str) -> Result<VpkPath> {
    Ok(VpkPath::new_with_prefix_and_extension(
        s,
        "materials",
        "vmt",
    ))
}

pub struct Vmt {
    path: VpkPath,
    shader: Shader,
}

impl Vmt {
    pub fn path(&self) -> &VpkPath {
        &self.path
    }

    pub fn shader(&self) -> &Shader {
        &self.shader
    }
}

impl Asset for Vmt {
    fn from_data(loader: &AssetLoader, path: &VpkPath, data: Vec<u8>) -> Result<Rc<Self>> {
        Ok(Rc::new(Self {
            path: path.clone(),
            shader: create_shader_builder(path, &data)?
                .build(loader, path)
                .with_context(|| format!("Building material {:?}", path.as_canonical_path()))?,
        }))
    }
}

pub enum Shader {
    LightmappedGeneric(LightmappedGeneric),
    Unsupported { shader: String },
}

trait ShaderBuilder<'a> {
    fn parse(&mut self, material_path: &VpkPath, entry: Entry<'a>) -> Result<()>;
    fn build(self: Box<Self>, loader: &AssetLoader, material_path: &VpkPath) -> Result<Shader>;
}

fn create_shader_builder<'a>(
    path: &VpkPath,
    data: &'a [u8],
) -> Result<Box<dyn ShaderBuilder<'a> + 'a>> {
    let root = crate::properties::vmt(from_utf8(&data)?).unwrap();

    let mut builder: Box<dyn ShaderBuilder<'a> + 'a> = match root.name {
        "LightmappedGeneric" => Box::new(LightmappedGenericBuilder::default()),
        "patch" => Box::new(PatchBuilder::default()),
        shader => {
            eprintln!("WARNING: Unimplemented shader {} in {}", shader, path);
            return Ok(Box::new(UnsupportedBuilder {
                shader: shader.to_string(),
            }));
        }
    };
    for entry in root.entries {
        builder
            .parse(path, entry)
            .with_context(|| format!("Parsing material {:?}", path.as_canonical_path()))?;
    }

    Ok(builder)
}

struct LightmappedGenericBuilder {
    alpha_test: bool,
    alpha_test_reference: f32,
    base_alpha_env_map_mask: bool,
    base_texture_path: Option<VpkPath>,
    bump_map_path: Option<VpkPath>,
    decal_path: Option<VpkPath>,
    detail_path: Option<VpkPath>,
    detail_blend_factor: f32,
    detail_blend_mode: i32,
    detail_scale: f32,
    env_map_path: Option<VpkPath>,
    env_map_contrast: Option<f32>,
    env_map_mask_path: Option<VpkPath>,
    env_map_saturation: Option<f32>,
    env_map_tint: Option<Vec3>,
    no_diffuse_bump_lighting: bool,
    normal_map_alpha_env_map_mask: bool,
    self_illum: bool,
    translucent: bool,
}

impl Default for LightmappedGenericBuilder {
    fn default() -> Self {
        Self {
            alpha_test_reference: 0.5, // ?
            alpha_test: false,
            base_alpha_env_map_mask: false,
            base_texture_path: None,
            bump_map_path: None,
            decal_path: None,
            detail_path: None,
            detail_blend_factor: 1.0, // ?
            detail_blend_mode: 0,
            detail_scale: 4.0,
            env_map_path: None,
            env_map_contrast: None,
            env_map_mask_path: None,
            env_map_saturation: None,
            env_map_tint: None,
            no_diffuse_bump_lighting: false,
            normal_map_alpha_env_map_mask: false,
            self_illum: false,
            translucent: false,
        }
    }
}

impl<'a> ShaderBuilder<'a> for LightmappedGenericBuilder {
    fn parse(&mut self, material_path: &VpkPath, entry: Entry<'a>) -> Result<()> {
        match entry {
            Entry::KeyValue(KeyValue { key, value }) => match key.to_ascii_lowercase().as_str() {
                "$alphatest" => self.alpha_test = parse_bool(value).context("$alphatest")?,
                "$alphatestreference" => {
                    self.alpha_test_reference = parse_f32(value).context("$alphatestreference")?
                }
                "$basealphaenvmapmask" => {
                    self.base_alpha_env_map_mask =
                        parse_bool(value).context("$basealphaenvmapmask")?
                }
                "$basetexture" => {
                    self.base_texture_path = parse_vtf_path(value).context("$basetexture")?
                }
                "$bumpmap" => self.bump_map_path = parse_vtf_path(value).context("$bumpmap")?,
                "$decal" => self.decal_path = parse_vtf_path(value).context("$decal")?,
                "$detail" => self.detail_path = parse_vtf_path(value).context("$detail")?,
                "$detailblendfactor" => {
                    self.detail_blend_factor = parse_f32(value).context("$detailblendfactor")?
                }
                "$detailblendmode" => {
                    self.detail_blend_mode = parse_i32(value).context("$detailblendmode")?
                }
                "$detailscale" => self.detail_scale = parse_f32(value).context("$detailscale")?,
                "$envmap" => self.env_map_path = parse_vtf_path(value).context("$envmap")?,
                "$envmapcontrast" => {
                    self.env_map_contrast = Some(parse_f32(value).context("$envmapcontrast")?)
                }
                "$envmapmask" => {
                    self.env_map_mask_path = parse_vtf_path(value).context("$envmapmask")?
                }
                "$envmapsaturation" => {
                    self.env_map_saturation =
                        Some(parse_vector_or_f32(value).context("$envmapsaturation")?)
                }
                "$envmaptint" => {
                    self.env_map_tint = Some(parse_material_vector(value).context("$envmaptint")?)
                }
                "$nodiffusebumplighting" => {
                    self.no_diffuse_bump_lighting =
                        parse_bool(value).context("$nodiffusebumplighting")?
                }
                "$normalmapalphaenvmapmask" => {
                    self.normal_map_alpha_env_map_mask =
                        parse_bool(value).context("$normalmapalphaenvmapmask")?
                }
                "$selfillum" => self.self_illum = parse_bool(value).context("$selfillum")?,
                "$translucent" => self.translucent = parse_bool(value).context("$translucent")?,
                "$parallaxmap" | "$parallaxmapscale" | "$reflectivity" | "$surfaceprop" => (),
                x if x.starts_with("%") => (),
                _ => eprintln!(
                    "WARNING: Unimplemented LightmappedGeneric key {} in {}",
                    key, material_path,
                ),
            },
            Entry::Object(Object { name, entries }) => match name.to_ascii_lowercase().as_str() {
                "proxies" => {
                    eprintln!(
                        "WARNING: Unimplemented material proxies in {}",
                        material_path,
                    );
                }

                // Fallbacks above the targeted dxlevel. Safe to completely ignore.
                "lightmappedgeneric_hdr_dx9"
                | "lightmappedgeneric_dx9"
                | "lightmappedgeneric_dx8"
                | "lightmappedgeneric_nobump_dx8" => (),

                // Fallback for the targeted dxlevel. Parse as if inlined in the main object.
                "lightmappedgeneric_dx6" => {
                    for entry in entries {
                        self.parse(material_path, entry)?;
                    }
                }

                name if name.contains(&['<', '>'][..]) => {
                    let operator = if name.starts_with("<=") {
                        "<="
                    } else if name.starts_with("<") {
                        "<"
                    } else if name.starts_with(">=") {
                        ">="
                    } else if name.starts_with(">") {
                        ">"
                    } else {
                        bail!(
                            "invalid conditional statement {:?} in {}",
                            name,
                            material_path,
                        );
                    };
                    let param = &name[operator.len()..];
                    match param {
                        "dx90" | "dx90_20b" => (),
                        _ => bail!(
                            "unexpected conditional value {:?} in {}",
                            param,
                            material_path,
                        ),
                    }

                    // Every valid param is above the target dxlevel.
                    match operator {
                        // Match. Parse as if inlined in the main object.
                        "<" | "<=" => {
                            for entry in entries {
                                self.parse(material_path, entry)?;
                            }
                        }

                        // No match. Safe to completely ignore.
                        ">=" | ">" => (),

                        _ => unreachable!(),
                    }
                }
                _ => eprintln!(
                    "WARNING: Unexpected LightmappedGeneric object {} in {}",
                    name, material_path,
                ),
            },
        }
        Ok(())
    }

    fn build(self: Box<Self>, _loader: &AssetLoader, material_path: &VpkPath) -> Result<Shader> {
        if self.env_map_path.is_some() && self.env_map_contrast != Some(1.0) {
            eprintln!(
                "WARNING: Unimplemented $envmapcontrast {:?} in {}",
                self.env_map_contrast, material_path,
            );
        }

        Ok(Shader::LightmappedGeneric(LightmappedGeneric {
            alpha_test: self.alpha_test,
            alpha_test_reference: self.alpha_test_reference,
            base_alpha_env_map_mask: self.base_alpha_env_map_mask,
            base_texture_path: match self.base_texture_path {
                Some(x) => x,
                None => bail!("LightmappedGeneric $basetexture was unset"),
            },
            bump_map_path: self.bump_map_path.clone(),
            decal_path: self.decal_path.clone(),
            detail_path: self.detail_path.clone(),
            detail_blend_factor: self.detail_blend_factor,
            detail_blend_mode: self.detail_blend_mode,
            detail_scale: self.detail_scale,
            env_map_path: self.env_map_path.clone(),
            env_map_contrast: self.env_map_contrast,
            env_map_mask_path: self.env_map_mask_path.clone(),
            env_map_saturation: self.env_map_saturation,
            env_map_tint: self.env_map_tint,
            no_diffuse_bump_lighting: self.no_diffuse_bump_lighting,
            normal_map_alpha_env_map_mask: self.normal_map_alpha_env_map_mask,
            self_illum: self.self_illum,
            translucent: self.translucent,
        }))
    }
}

#[derive(Clone, Debug)]
pub struct LightmappedGeneric {
    pub alpha_test_reference: f32,
    pub alpha_test: bool,
    pub base_alpha_env_map_mask: bool,
    pub base_texture_path: VpkPath,
    pub bump_map_path: Option<VpkPath>,
    pub decal_path: Option<VpkPath>,
    pub detail_path: Option<VpkPath>,
    pub detail_blend_factor: f32,
    pub detail_blend_mode: i32,
    pub detail_scale: f32,
    pub env_map_path: Option<VpkPath>,
    pub env_map_contrast: Option<f32>,
    pub env_map_mask_path: Option<VpkPath>,
    pub env_map_saturation: Option<f32>,
    pub env_map_tint: Option<Vec3>,
    pub no_diffuse_bump_lighting: bool,
    pub normal_map_alpha_env_map_mask: bool,
    pub self_illum: bool,
    pub translucent: bool,
}

#[derive(Default)]
pub struct PatchBuilder<'a> {
    include: Option<VpkPath>,
    entries: Vec<Entry<'a>>,
}

impl<'a> ShaderBuilder<'a> for PatchBuilder<'a> {
    fn parse(&mut self, material_path: &VpkPath, entry: Entry<'a>) -> Result<()> {
        match entry {
            Entry::KeyValue(KeyValue { key, value }) => match key.to_ascii_lowercase().as_str() {
                "include" => self.include = Some(parse_vmt_path(value)?),
                _ => eprintln!("WARNING: Unexpected patch key {} in {}", key, material_path),
            },
            Entry::Object(Object { name, mut entries }) => match name.to_ascii_lowercase().as_str()
            {
                "replace" => self.entries.append(&mut entries),
                _ => eprintln!(
                    "WARNING: Unexpected patch object {} in {}",
                    name, material_path,
                ),
            },
        }
        Ok(())
    }

    fn build(self: Box<Self>, loader: &AssetLoader, material_path: &VpkPath) -> Result<Shader> {
        let include_path = match self.include {
            Some(include_path) => include_path,
            None => bail!("patch material without include parameter"),
        };
        let data = loader.material_loader().load_file(&include_path)?.unwrap();
        let mut builder = create_shader_builder(&include_path, &data)?;

        for entry in self.entries {
            builder.parse(material_path, entry)?;
        }

        builder.build(loader, material_path)
    }
}

pub struct UnsupportedBuilder {
    shader: String,
}

impl ShaderBuilder<'_> for UnsupportedBuilder {
    fn parse(&mut self, _material_path: &VpkPath, _entry: Entry) -> Result<()> {
        Ok(())
    }

    fn build(self: Box<Self>, _loader: &AssetLoader, _material_path: &VpkPath) -> Result<Shader> {
        Ok(Shader::Unsupported {
            shader: self.shader.clone(),
        })
    }
}
