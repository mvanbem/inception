use std::rc::Rc;
use std::str::from_utf8;

use anyhow::{bail, Context, Result};
use nalgebra_glm::Vec3;
use try_map::FallibleMapExt;

use crate::asset::vmt::parse::{Entry, KeyValue, Object};
use crate::asset::vtf::Vtf;
use crate::asset::{Asset, AssetLoader};
use crate::vpk::path::VpkPath;

mod parse;

fn parse_bool(s: &str) -> Result<bool> {
    match s {
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
    match parse::material_vector(s) {
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
        let root = parse::vmt(from_utf8(&data)?).unwrap();

        let mut builder: Box<dyn ShaderBuilder> = match root.name {
            "LightmappedGeneric" => Box::new(LightmappedGenericBuilder::default()),
            "patch" => Box::new(PatchBuilder::default()),
            "UnlitGeneric" | "Water" | "WorldVertexTransition" => {
                return Ok(Rc::new(Self {
                    path: path.clone(),
                    shader: Shader::Unsupported,
                }))
            }
            _ => panic!("unexpected shader: {}", root.name),
        };
        for entry in root.entries {
            builder
                .parse(entry)
                .with_context(|| format!("Parsing material {:?}", path.as_canonical_path()))?;
        }

        Ok(Rc::new(Self {
            path: path.clone(),
            shader: builder
                .build(loader)
                .with_context(|| format!("Building material {:?}", path.as_canonical_path()))?,
        }))
    }
}

pub enum Shader {
    LightmappedGeneric(LightmappedGeneric),
    Unsupported,
}

impl Shader {
    fn to_builder<'a>(&self) -> Result<Box<dyn ShaderBuilder<'a> + 'a>> {
        match self {
            Shader::LightmappedGeneric(shader) => shader.to_builder(),
            Shader::Unsupported => bail!("can't make a builder for an unsupported shader"),
        }
    }
}

trait ShaderBuilder<'a> {
    fn parse(&mut self, entry: Entry<'a>) -> Result<()>;
    fn build(self: Box<Self>, loader: &AssetLoader) -> Result<Shader>;
}

struct LightmappedGenericBuilder {
    alpha_test: bool,
    alpha_test_reference: f32,
    base_alpha_env_map_mask: bool,
    base_texture: Option<VpkPath>,
    bump_map: Option<VpkPath>,
    decal: Option<VpkPath>,
    detail: Option<VpkPath>,
    detail_blend_factor: f32,
    detail_blend_mode: i32,
    detail_scale: f32,
    env_map: Option<VpkPath>,
    env_map_contrast: Option<f32>,
    env_map_mask: Option<VpkPath>,
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
            base_texture: None,
            bump_map: None,
            decal: None,
            detail: None,
            detail_blend_factor: 1.0, // ?
            detail_blend_mode: 0,
            detail_scale: 4.0,
            env_map: None,
            env_map_contrast: None,
            env_map_mask: None,
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
    fn parse(&mut self, entry: Entry) -> Result<()> {
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
                    self.base_texture = parse_vtf_path(value).context("$basetexture")?
                }
                "$bumpmap" => self.bump_map = parse_vtf_path(value).context("$bumpmap")?,
                "$decal" => self.decal = parse_vtf_path(value).context("$decal")?,
                "$detail" => self.detail = parse_vtf_path(value).context("$detail")?,
                "$detailblendfactor" => {
                    self.detail_blend_factor = parse_f32(value).context("$detailblendfactor")?
                }
                "$detailblendmode" => {
                    self.detail_blend_mode = parse_i32(value).context("$detailblendmode")?
                }
                "$detailscale" => self.detail_scale = parse_f32(value).context("$detailscale")?,
                "$envmap" => self.env_map = parse_vtf_path(value).context("$envmap")?,
                "$envmapcontrast" => {
                    self.env_map_contrast = Some(parse_f32(value).context("$envmapcontrast")?)
                }
                "$envmapmask" => {
                    self.env_map_mask = parse_vtf_path(value).context("$envmapmask")?
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
                _ => println!("unexpected LightmappedGeneric key: {}", key),
            },
            Entry::Object(Object { name, .. }) => match name.to_ascii_lowercase().as_str() {
                "proxies" => println!("ignoring unsupported material proxy"),
                name if name.ends_with("_dx8")
                    || name.ends_with("_dx9")
                    || name.contains("_hdr_") =>
                {
                    ()
                }
                _ => println!("unexpected LightmappedGeneric object: {}", name),
            },
        }
        Ok(())
    }

    fn build(self: Box<Self>, loader: &AssetLoader) -> Result<Shader> {
        Ok(Shader::LightmappedGeneric(LightmappedGeneric {
            alpha_test: self.alpha_test,
            alpha_test_reference: self.alpha_test_reference,
            base_alpha_env_map_mask: self.base_alpha_env_map_mask,
            base_texture: match self.base_texture {
                Some(x) => loader.get_texture(&x)?,
                None => bail!("LightmappedGeneric $basetexture was unset"),
            },
            bump_map: self.bump_map.try_map(|path| loader.get_texture(&path))?,
            decal: self.decal.try_map(|path| loader.get_texture(&path))?,
            detail: self.detail.try_map(|path| loader.get_texture(&path))?,
            detail_blend_factor: self.detail_blend_factor,
            detail_blend_mode: self.detail_blend_mode,
            detail_scale: self.detail_scale,
            env_map: self.env_map.try_map(|path| loader.get_texture(&path))?,
            env_map_contrast: self.env_map_contrast,
            env_map_mask: self
                .env_map_mask
                .try_map(|path| loader.get_texture(&path))?,
            env_map_saturation: self.env_map_saturation,
            env_map_tint: self.env_map_tint,
            no_diffuse_bump_lighting: self.no_diffuse_bump_lighting,
            normal_map_alpha_env_map_mask: self.normal_map_alpha_env_map_mask,
            self_illum: self.self_illum,
            translucent: self.translucent,
        }))
    }
}

pub struct LightmappedGeneric {
    pub alpha_test_reference: f32,
    pub alpha_test: bool,
    pub base_alpha_env_map_mask: bool,
    pub base_texture: Rc<Vtf>,
    pub bump_map: Option<Rc<Vtf>>,
    pub decal: Option<Rc<Vtf>>,
    pub detail: Option<Rc<Vtf>>,
    pub detail_blend_factor: f32,
    pub detail_blend_mode: i32,
    pub detail_scale: f32,
    pub env_map: Option<Rc<Vtf>>,
    pub env_map_contrast: Option<f32>,
    pub env_map_mask: Option<Rc<Vtf>>,
    pub env_map_saturation: Option<f32>,
    pub env_map_tint: Option<Vec3>,
    pub no_diffuse_bump_lighting: bool,
    pub normal_map_alpha_env_map_mask: bool,
    pub self_illum: bool,
    pub translucent: bool,
}

impl LightmappedGeneric {
    fn to_builder<'a>(&self) -> Result<Box<dyn ShaderBuilder<'a> + 'a>> {
        Ok(Box::new(LightmappedGenericBuilder {
            alpha_test_reference: self.alpha_test_reference,
            alpha_test: self.alpha_test,
            base_alpha_env_map_mask: self.base_alpha_env_map_mask,
            base_texture: Some(self.base_texture.path().clone()),
            bump_map: self.bump_map.as_ref().map(|vtf| vtf.path().clone()),
            decal: self.decal.as_ref().map(|vtf| vtf.path().clone()),
            detail: self.detail.as_ref().map(|vtf| vtf.path().clone()),
            detail_blend_factor: self.detail_blend_factor,
            detail_blend_mode: self.detail_blend_mode,
            detail_scale: self.detail_scale,
            env_map: self.env_map.as_ref().map(|vtf| vtf.path().clone()),
            env_map_contrast: self.env_map_contrast,
            env_map_mask: self.env_map_mask.as_ref().map(|vtf| vtf.path().clone()),
            env_map_saturation: self.env_map_saturation,
            env_map_tint: self.env_map_tint,
            no_diffuse_bump_lighting: self.no_diffuse_bump_lighting,
            normal_map_alpha_env_map_mask: self.normal_map_alpha_env_map_mask,
            self_illum: self.self_illum,
            translucent: self.translucent,
        }))
    }
}

#[derive(Default)]
pub struct PatchBuilder<'a> {
    include: Option<VpkPath>,
    entries: Vec<Entry<'a>>,
}

impl<'a> ShaderBuilder<'a> for PatchBuilder<'a> {
    fn parse(&mut self, entry: Entry<'a>) -> Result<()> {
        match entry {
            Entry::KeyValue(KeyValue { key, value }) => match key.to_ascii_lowercase().as_str() {
                "include" => self.include = Some(parse_vmt_path(value)?),
                _ => println!("unexpected patch key: {}", key),
            },
            Entry::Object(Object { name, mut entries }) => match name.to_ascii_lowercase().as_str()
            {
                "replace" => self.entries.append(&mut entries),
                _ => println!("unexpected patch object: {}", name),
            },
        }
        Ok(())
    }

    fn build(self: Box<Self>, loader: &AssetLoader) -> Result<Shader> {
        let mut builder = match self.include {
            Some(x) => loader.get_material(&x)?.shader.to_builder()?,
            None => bail!("patch material without include parameter"),
        };

        for entry in self.entries {
            builder.parse(entry)?;
        }

        builder.build(loader)
    }
}
