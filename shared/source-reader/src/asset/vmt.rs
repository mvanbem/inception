use std::rc::Rc;
use std::str::from_utf8;

use anyhow::{bail, Result};

use crate::asset::vmt::parse::{Entry, KeyValue, Object};
use crate::asset::vtf::Vtf;
use crate::asset::{Asset, AssetLoader};
use crate::vpk::path::VpkPath;

mod parse;

fn set_if_none_or_die<T>(param: &str, opt: &mut Option<T>, value: T) {
    match opt {
        Some(_) => panic!("duplicate {} entry", param),
        None => *opt = Some(value),
    }
}

macro_rules! match_params {
    ($key:ident $value:ident $loader:ident { $($param:literal: $type_:tt => $var:ident,)* }) => {
        {
            let key = $key.to_ascii_lowercase();
            match key.as_str() {
                x if x.starts_with('%') => (),
                $($param => match_params_action!($value $loader $param $type_ $var),)*
                _ => println!("unexpected key: {}", $key),
            }
        }
    };
}

macro_rules! match_params_action {
    ($value:ident $loader:ident $param:literal () $var:ident) => {
        ()
    };
    ($value:ident $loader:ident $param:literal bool $var:ident) => {
        set_if_none_or_die(
            $param,
            &mut $var,
            match $value {
                "1" => true,
                _ => $value
                    .parse()
                    .unwrap_or_else(|_| panic!(concat!($param, " bad bool parameter: {}"), $value)),
            },
        )
    };
    ($value:ident $loader:ident $param:literal i32 $var:ident) => {
        set_if_none_or_die(
            $param,
            &mut $var,
            $value
                .parse()
                .unwrap_or_else(|_| panic!(concat!($param, " bad i32 parameter: {}"), $value)),
        )
    };
    ($value:ident $loader:ident $param:literal f32 $var:ident) => {
        set_if_none_or_die(
            $param,
            &mut $var,
            $value
                .parse()
                .unwrap_or_else(|_| panic!(concat!($param, " bad f32 parameter: {}"), $value)),
        )
    };
    ($value:ident $loader:ident $param:literal VpkPath $var:ident) => {
        set_if_none_or_die(
            $param,
            &mut $var,
            $loader.get_texture(&VpkPath::new_with_prefix_and_extension(
                $value,
                "materials",
                "vtf",
            ))?,
        )
    };
}

pub struct Vmt {
    shader: Shader,
}

impl Vmt {
    pub fn shader(&self) -> &Shader {
        &self.shader
    }
}

impl Asset for Vmt {
    fn from_data(loader: &AssetLoader, _path: &VpkPath, data: Vec<u8>) -> Result<Rc<Self>> {
        let root = parse::vmt(from_utf8(&data)?).unwrap();

        let shader = match root.name {
            "LightmappedGeneric" => {
                let mut alpha_test = None;
                let mut alpha_test_reference = None;
                let mut base_texture = None;
                let mut bump_map = None;
                let mut decal = None;
                let mut detail = None;
                let mut detail_blend_factor = None;
                let mut detail_blend_mode = None;
                let mut detail_scale = None;
                let mut self_illum = None;
                let mut translucent = None;
                for entry in root.entries {
                    match entry {
                        Entry::KeyValue(KeyValue { key, value }) => {
                            match_params!(key value loader {
                                "$alphatest": bool => alpha_test,
                                "$alphatestreference": f32 => alpha_test_reference,
                                "$basetexture": VpkPath => base_texture,
                                "$bumpmap": VpkPath => bump_map,
                                "$decal": VpkPath => decal,
                                "$detail": VpkPath => detail,
                                "$detailblendfactor": f32 => detail_blend_factor,
                                "$detailblendmode": i32 => detail_blend_mode,
                                "$detailscale": i32 => detail_scale,
                                "$selfillum": bool => self_illum,
                                "$translucent": bool => translucent,

                                "$parallaxmap": () => ignored,
                                "$parallaxmapscale": () => ignored,
                                "$surfaceprop": () => ignored,
                            })
                        }
                        Entry::Object(Object { name, .. }) => match name.to_lowercase().as_str() {
                            "proxies" => println!("ignoring unsupported material proxy"),
                            _ => println!("unexpected LightmappedGeneric object: {}", name),
                        },
                    }
                }
                Shader::LightmappedGeneric {
                    alpha_test: alpha_test.unwrap_or(false),
                    // TODO: Verify this default!
                    alpha_test_reference: alpha_test_reference.unwrap_or(0.5),
                    bump_map,
                    base_texture: base_texture.expect("LightmappedGeneric $basetexture was unset"),
                    decal,
                    detail,
                    detail_blend_factor,
                    detail_blend_mode,
                    detail_scale,
                    self_illum: self_illum.unwrap_or(false),
                    translucent: translucent.unwrap_or(false),
                }
            }
            "patch" => {
                for entry in root.entries {
                    match entry {
                        Entry::KeyValue(KeyValue { key, value }) => match key {
                            "include" => {
                                return loader.get_material(
                                    &VpkPath::new_with_prefix_and_extension(
                                        value,
                                        "materials",
                                        "vmt",
                                    ),
                                );
                            }
                            _ => (),
                        },
                        Entry::Object(_) => (),
                    }
                }
                bail!("patch material without include parameter")
            }
            "UnlitGeneric" | "Water" | "WorldVertexTransition" => Shader::Unsupported,
            _ => panic!("unexpected shader: {}", root.name),
        };

        Ok(Rc::new(Self { shader }))
    }
}

pub enum Shader {
    LightmappedGeneric {
        alpha_test: bool,
        alpha_test_reference: f32,
        bump_map: Option<Rc<Vtf>>,
        base_texture: Rc<Vtf>,
        decal: Option<Rc<Vtf>>,
        detail: Option<Rc<Vtf>>,
        detail_blend_factor: Option<f32>,
        detail_blend_mode: Option<i32>,
        detail_scale: Option<f32>,
        self_illum: bool,
        translucent: bool,
    },
    Unsupported,
}
