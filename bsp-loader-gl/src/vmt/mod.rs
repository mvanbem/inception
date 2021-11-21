use std::str::from_utf8;

use crate::file::canonical_path::CanonicalPathBuf;
use crate::vmt::parse::{Entry, KeyValue, Object};

mod parse;

fn set_if_none_or_die<T>(param: &str, opt: &mut Option<T>, value: T) {
    match opt {
        Some(_) => panic!("duplicate {} entry", param),
        None => *opt = Some(value),
    }
}

macro_rules! match_params {
    ($key:ident $value:ident { $($param:literal: $type_:tt => $var:ident,)* }) => {
        {
            let key = $key.to_ascii_lowercase();
            match key.as_str() {
                x if x.starts_with('%') => (),
                $($param => match_params_action!($value $param $type_ $var),)*
                _ => println!("unexpected key: {}", $key),
            }
        }
    };
}

macro_rules! match_params_action {
    ($value:ident $param:literal () $var:ident) => {
        ()
    };
    ($value:ident $param:literal bool $var:ident) => {
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
    ($value:ident $param:literal i32 $var:ident) => {
        set_if_none_or_die(
            $param,
            &mut $var,
            $value
                .parse()
                .unwrap_or_else(|_| panic!(concat!($param, " bad i32 parameter: {}"), $value)),
        )
    };
    ($value:ident $param:literal f32 $var:ident) => {
        set_if_none_or_die(
            $param,
            &mut $var,
            $value
                .parse()
                .unwrap_or_else(|_| panic!(concat!($param, " bad f32 parameter: {}"), $value)),
        )
    };
    ($value:ident $param:literal CanonicalPathBuf $var:ident) => {
        set_if_none_or_die(
            $param,
            &mut $var,
            CanonicalPathBuf::from_str_canonicalize($value),
        )
    };
}

pub struct Vmt {
    shader: Shader,
}

impl Vmt {
    pub fn new(data: &[u8]) -> Self {
        let root = parse::vmt(from_utf8(data).unwrap()).unwrap();

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
                        Entry::KeyValue(KeyValue { key, value }) => match_params!(key value {
                            "$alphatest": bool => alpha_test,
                            "$alphatestreference": f32 => alpha_test_reference,
                            "$basetexture": CanonicalPathBuf => base_texture,
                            "$bumpmap": CanonicalPathBuf => bump_map,
                            "$decal": CanonicalPathBuf => decal,
                            "$detail": CanonicalPathBuf => detail,
                            "$detailblendfactor": f32 => detail_blend_factor,
                            "$detailblendmode": i32 => detail_blend_mode,
                            "$detailscale": i32 => detail_scale,
                            "$parallaxmap": () => ignored,
                            "$parallaxmapscale": () => ignored,
                            "$selfillum": bool => self_illum,
                            "$surfaceprop": () => ignored,
                            "$translucent": bool => translucent,
                        }),
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
            "patch" | "UnlitGeneric" | "WorldVertexTransition" => Shader::Unsupported,
            _ => panic!("unexpected shader: {}", root.name),
        };

        Self { shader }
    }
}

enum Shader {
    LightmappedGeneric {
        alpha_test: bool,
        alpha_test_reference: f32,
        bump_map: Option<CanonicalPathBuf>,
        base_texture: CanonicalPathBuf,
        decal: Option<CanonicalPathBuf>,
        detail: Option<CanonicalPathBuf>,
        detail_blend_factor: Option<f32>,
        detail_blend_mode: Option<i32>,
        detail_scale: Option<f32>,
        self_illum: bool,
        translucent: bool,
    },
    Unsupported,
}
