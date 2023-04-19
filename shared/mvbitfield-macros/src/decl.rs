use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::spanned::Spanned;
use syn::{Attribute, Error, Ident, Path, Result, Type, TypePath, Visibility};

use crate::input::{self, AccessorType, Bitfield, Input};
use crate::types::{
    convert, BoolType, BorrowedType, NarrowIntegerType, OwnedType, UserType, UserTypeRepr,
};

fn lsb_mask(width: usize) -> usize {
    (1 << width) - 1
}

fn lsb_offset_mask(offset: usize, width: usize) -> usize {
    lsb_mask(width) << offset
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackDir {
    LsbFirst,
    MsbFirst,
}

pub struct Config {
    pub crate_path: Path,
}

impl Config {
    pub fn from_ast(ast: &Input) -> Self {
        Self {
            crate_path: ast.crate_path.clone(),
        }
    }
}

pub(crate) struct StructDecl<'a> {
    cfg: &'a Config,
    header: StructHeader<'a>,
    bitfields: Vec<&'a Bitfield>,
}

impl<'a> StructDecl<'a> {
    pub fn from_ast(cfg: &'a Config, input: &'a input::Struct) -> Result<Self> {
        Ok(StructDecl {
            cfg,
            header: StructHeader::from_ast(
                cfg,
                &input.attrs,
                &input.visibility,
                &input.name,
                &input.underlying_type,
            )?,
            bitfields: input.bitfields.iter().collect(),
        })
    }

    pub fn into_token_stream(self) -> Result<TokenStream> {
        let Self {
            cfg,
            header,
            bitfields,
        } = self;

        // Collect non-bitfield items.
        let zero_method = header.make_zero_method(cfg);
        let from_repr_method = header.make_from_repr_method();
        let as_repr_method = header.make_as_repr_method();

        // Collect bitfield items.
        let bitfield_items = pack(&header, &bitfields)?
            .into_iter()
            .map(|bitfield| BitfieldCodegenContext::new(cfg, &header, bitfield))
            .collect::<Result<Vec<_>>>()?;

        let name = &header.name;
        let repr = header.underlying_type.to_token_stream();

        // Collect additional impl blocks.
        let mut additional_impls = Vec::new();
        if matches!(header.underlying_type, OwnedType::PrimitiveInteger(_)) {
            additional_impls.push(quote! {
                impl ::core::convert::From<#repr> for #name {
                    fn from(value: #repr) -> Self {
                        Self { value }
                    }
                }

                impl ::core::convert::From<#name> for #repr {
                    fn from(value: #name) -> Self {
                        value.value
                    }
                }
            });
        }

        // Emit the struct and impl block.
        let other_attrs = &header.other_attrs;
        let visibility = &header.visibility;
        Ok(quote! {
            #[derive(
                ::core::clone::Clone,
                ::core::marker::Copy,
                ::core::fmt::Debug,
            )]
            #[repr(transparent)]
            #(#other_attrs)*
            #visibility struct #name {
                value: #repr,
            }

            #[allow(dead_code)]
            impl #name {
                #zero_method
                #from_repr_method
                #as_repr_method

                #(#bitfield_items)*
            }

            #(#additional_impls)*
        })
    }
}

struct StructHeader<'a> {
    pack_dir: Option<PackDir>,
    other_attrs: Vec<&'a Attribute>,
    visibility: &'a Visibility,
    name: &'a Ident,
    underlying_type: OwnedType,
}

impl<'a> StructHeader<'a> {
    fn from_ast(
        cfg: &Config,
        attrs: &'a [Attribute],
        visibility: &'a Visibility,
        name: &'a Ident,
        repr: &'a Type,
    ) -> Result<Self> {
        let mut pack_dir = None;
        let mut other_attrs = Vec::new();

        for attr in attrs {
            match attr.path() {
                path if path.is_ident("lsb_first") => {
                    attr.meta.require_path_only()?;
                    if pack_dir.is_some() {
                        return Err(Error::new(
                            attr.span(),
                            "multiple packing direction attributes are not allowed",
                        ));
                    }
                    pack_dir = Some(PackDir::LsbFirst);
                }
                path if path.is_ident("msb_first") => {
                    attr.meta.require_path_only()?;
                    if pack_dir.is_some() {
                        return Err(Error::new(
                            attr.span(),
                            "multiple packing direction attributes are not allowed",
                        ));
                    }
                    pack_dir = Some(PackDir::MsbFirst);
                }
                _ => other_attrs.push(attr),
            }
        }

        Ok(Self {
            pack_dir,
            other_attrs,
            visibility,
            name,
            underlying_type: OwnedType::from_type(cfg, &repr),
        })
    }

    fn make_from_repr_method(&self) -> TokenStream {
        let method = format_ident!("from_{}", self.underlying_type.to_method_name_snippet());
        let repr = self.underlying_type.to_token_stream();
        quote! {
            #[doc = "Constructs the bitfield struct from its underlying type."]
            #[inline(always)]
            #[must_use]
            pub const fn #method(value: #repr) -> Self {
                Self { value }
            }
        }
    }

    fn make_as_repr_method(&self) -> TokenStream {
        let method = format_ident!("as_{}", self.underlying_type.to_method_name_snippet());
        let repr = self.underlying_type.to_token_stream();
        quote! {
            #[doc = "Converts the bitfield struct to its underlying type."]
            #[inline(always)]
            #[must_use]
            pub const fn #method(self) -> #repr {
                self.value
            }
        }
    }

    fn make_zero_method(&self, settings: &Config) -> TokenStream {
        let expr = match &self.underlying_type {
            OwnedType::PrimitiveInteger(_) => quote! { 0 },
            OwnedType::NarrowInteger(NarrowIntegerType { path, .. }) => {
                let crate_path = &settings.crate_path;
                quote! { <#path as #crate_path::narrow_integer::NarrowInteger>::ZERO }
            }
            _ => panic!(),
        };
        quote! {
            #[doc = "The zero value for this type."]
            #[inline(always)]
            #[must_use]
            pub const fn zero() -> Self {
                Self { value: #expr }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct PackedBitfield<'a> {
    input: &'a Bitfield,
    offset: usize,
    width: usize,
}

fn pack<'a>(header: &StructHeader, bitfields: &[&'a Bitfield]) -> Result<Vec<PackedBitfield<'a>>> {
    let pack_dir = match header.pack_dir {
        Some(pack_dir) => pack_dir,
        None if bitfields.len() < 2 => PackDir::LsbFirst, // Doesn't matter, just pick one.
        None => {
            return Err(Error::new(
                header.name.span(),
                "a packing direction attribute is required (`#[lsb_first]` or `#[msb_first]`)",
            ))
        }
    };

    #[derive(Clone, Copy)]
    struct KnownWidthBitfield<'a> {
        input: &'a Bitfield,
        width: usize,
    }
    let mut bitfields_before_flexible = Vec::new();
    let mut flexible_bitfield = None;
    let mut bitfields_after_flexible = Vec::new();

    // Initial pass: Collect placeholders into each list of fields in the order they will be packed.
    // Verify there are zero or one flexible bitfields.
    for bitfield in bitfields {
        match bitfield.width()? {
            Some(width) => {
                let dst = if flexible_bitfield.is_none() {
                    &mut bitfields_before_flexible
                } else {
                    &mut bitfields_after_flexible
                };
                dst.push(KnownWidthBitfield {
                    input: bitfield,
                    width: width as usize,
                });
            }
            None => {
                if flexible_bitfield.is_some() {
                    return Err(Error::new(
                        bitfield.name_span(),
                        "only up to one flexible bitfield is permitted",
                    ));
                } else {
                    flexible_bitfield = Some(bitfield);
                }
            }
        }
    }

    // Compute available bits after considering all sized bitfields.
    let mut available = header.underlying_type.width();
    for bitfield in bitfields_before_flexible
        .iter()
        .chain(bitfields_after_flexible.iter())
    {
        if let Some(new_available) = available.checked_sub(bitfield.width) {
            available = new_available;
        } else {
            return Err(Error::new(
                bitfield.input.name_span(),
                format!("bitfield overflows containing struct; {available} bit(s) available"),
            ));
        }
    }

    // Size the flexible bitfield, if present.
    let flexible_bitfield = match flexible_bitfield {
        Some(input) if available > 0 => Some(KnownWidthBitfield {
            input,
            width: available,
        }),
        Some(input) => {
            return Err(Error::new(
                input.name_span(),
                format!("no bits available for flexible bitfield"),
            ))
        }
        None if available == 0 => None,
        None => {
            return Err(Error::new(
                header.name.span(),
                format!(
                    "there are {available} unassigned bit(s); consider specifying an anonymous \
                        flexible bitfield `..` if this is intended",
                ),
            ))
        }
    };

    // The bitfields are known to fit and are all sized. Pack them.
    let mut lsb_offset = match pack_dir {
        PackDir::LsbFirst => 0,
        PackDir::MsbFirst => header.underlying_type.width(),
    };
    let mut packed = Vec::new();
    for bitfield in bitfields_before_flexible
        .iter()
        .chain(flexible_bitfield.iter())
        .chain(bitfields_after_flexible.iter())
    {
        packed.push(PackedBitfield {
            input: bitfield.input,
            offset: match pack_dir {
                PackDir::LsbFirst => {
                    let this_lsb_offset = lsb_offset;
                    lsb_offset += bitfield.width;
                    this_lsb_offset
                }
                PackDir::MsbFirst => {
                    lsb_offset -= bitfield.width;
                    lsb_offset
                }
            },
            width: bitfield.width,
        });
    }
    Ok(packed)
}

struct BitfieldCodegenContext<'a> {
    bitfield: PackedBitfield<'a>,

    header_repr_type: BorrowedType<'a>,
    header_work_type: BorrowedType<'a>,
    name: String,
    accessor_type: OwnedType,

    get_method: Ident,
    with_method: Ident,
    map_method: Ident,
    set_method: Ident,
    modify_method: Ident,
}

impl<'a> BitfieldCodegenContext<'a> {
    fn new(
        cfg: &'a Config,
        header: &'a StructHeader,
        bitfield: PackedBitfield<'a>,
    ) -> Result<Option<Self>> {
        // Reserved fields do not generate any code.
        let name = bitfield.input.name_to_string();
        if name.starts_with('_') {
            return Ok(None);
        }

        let accessor_type = match bitfield.input.accessor_type() {
            AccessorType::Overridden { type_, .. } => match type_ {
                Type::Path(TypePath { qself: None, path }) => {
                    if path.is_ident("bool") {
                        OwnedType::Bool(BoolType {
                            ident: path.get_ident().unwrap().clone(),
                        })
                    } else {
                        OwnedType::User(UserType {
                            repr: match OwnedType::new_integer_span(
                                cfg,
                                bitfield.width,
                                bitfield.input.width_span(),
                            ) {
                                OwnedType::PrimitiveInteger(t) => UserTypeRepr::PrimitiveInteger(t),
                                OwnedType::NarrowInteger(t) => UserTypeRepr::NarrowInteger(t),
                                _ => unreachable!(),
                            },
                            path,
                        })
                    }
                }
                _ => {
                    return Err(Error::new(
                        type_.span(),
                        "accessor type override must be a path with no qualified self",
                    ));
                }
            },
            AccessorType::Default => {
                OwnedType::new_integer_span(cfg, bitfield.width, bitfield.input.width_span())
            }
        };
        let name = bitfield.input.name_to_string();
        let name_span = bitfield.input.name_span();
        let get_method = format_ident!("{name}", span = name_span);
        let with_method = format_ident!("with_{name}", span = name_span);
        let map_method = format_ident!("map_{name}", span = name_span);
        let set_method = format_ident!("set_{name}", span = name_span);
        let modify_method = format_ident!("modify_{name}", span = name_span);
        Ok(Some(Self {
            bitfield,

            header_repr_type: header.underlying_type.to_borrowed(),
            header_work_type: header.underlying_type.to_primitive(),
            name,
            accessor_type,

            get_method,
            with_method,
            map_method,
            set_method,
            modify_method,
        }))
    }

    fn make_get_method(&self) -> TokenStream {
        let shift = Literal::usize_unsuffixed(self.bitfield.offset);
        let lsb_mask = Literal::usize_unsuffixed(lsb_mask(self.bitfield.width));

        let doc = format!("Extracts the `{}` field.", self.name);
        let visibility = &self.bitfield.input.visibility;
        let get_method_name = &self.get_method;
        let accessor_type = &self.accessor_type;
        let result_io = {
            let self_value_work = convert(
                quote! { self.value },
                self.header_repr_type,
                self.header_work_type,
            );
            convert(
                quote! { ((#self_value_work) >> #shift) & #lsb_mask },
                self.header_work_type,
                self.accessor_type.to_borrowed(),
            )
        };
        quote! {
            #[doc = #doc]
            #[inline(always)]
            #[must_use]
            #visibility const fn #get_method_name(self) -> #accessor_type {
                #result_io
            }
        }
    }

    fn make_with_method(&self) -> TokenStream {
        let offset_mask =
            Literal::usize_unsuffixed(lsb_offset_mask(self.bitfield.offset, self.bitfield.width));
        let shift = Literal::usize_unsuffixed(self.bitfield.offset);

        let doc = format!(
            "Returns a new value with the `{}` field inserted.",
            self.name,
        );
        let visibility = &self.bitfield.input.visibility;
        let with_method_name = &self.with_method;
        let accessor_type = &self.accessor_type;
        let field_work = convert(
            quote! { value },
            self.accessor_type.to_borrowed(),
            self.header_work_type,
        );
        let result_repr = {
            let self_value_work = convert(
                quote! { self.value },
                self.header_repr_type,
                self.header_work_type,
            );
            convert(
                quote! { ((#self_value_work) & !#offset_mask) | field },
                self.header_work_type,
                self.header_repr_type,
            )
        };
        quote! {
            #[doc = #doc]
            #[inline(always)]
            #[must_use]
            #visibility const fn #with_method_name(self, value: #accessor_type) -> Self {
                let field = (#field_work) << #shift;

                // This is a redundant operation but it helps the compiler emit the `rlwimi`
                // instruction on PowerPC.
                #[cfg(target_arch = "powerpc")]
                let field = field & #offset_mask;

                Self { value: #result_repr }
            }
        }
    }

    fn make_map_method(&self) -> TokenStream {
        let doc = format!("Returns a new value with the `{}` field mapped.", self.name,);
        let visibility = &self.bitfield.input.visibility;
        let map_method = &self.map_method;
        let accessor_type = &self.accessor_type;
        let with_method = &self.with_method;
        let get_method = &self.get_method;
        quote! {
            #[doc = #doc]
            #[inline(always)]
            #[must_use]
            #visibility fn #map_method(self, f: impl ::core::ops::FnOnce(#accessor_type) -> #accessor_type) -> Self {
                self.#with_method(f(self.#get_method()))
            }
        }
    }

    fn make_set_method(&self) -> TokenStream {
        let offset_mask =
            Literal::usize_unsuffixed(lsb_offset_mask(self.bitfield.offset, self.bitfield.width));
        let shift = Literal::usize_unsuffixed(self.bitfield.offset);

        let doc = format!("Inserts the `{}` field.", self.name);
        let visibility = &self.bitfield.input.visibility;
        let set_method = &self.set_method;
        let accessor_type = &self.accessor_type;
        let value_work = convert(
            quote! { value },
            self.accessor_type.to_borrowed(),
            self.header_work_type,
        );
        let result_repr = {
            let self_value_work = convert(
                quote! { self.value },
                self.header_repr_type,
                self.header_work_type,
            );
            convert(
                quote! { ((#self_value_work) & !#offset_mask) | field },
                self.header_work_type,
                self.header_repr_type,
            )
        };
        quote! {
            #[doc = #doc]
            #[inline(always)]
            #visibility fn #set_method(&mut self, value: #accessor_type) {
                let field = (#value_work) << #shift;

                // This is a redundant operation but it helps the compiler emit the `rlwimi`
                // instruction on PowerPC.
                #[cfg(target_arch = "powerpc")]
                let field = field & #offset_mask;

                self.value = #result_repr;
            }
        }
    }

    fn make_modify_method(&self) -> TokenStream {
        let get_method = &self.get_method;
        let set_method = &self.set_method;

        let doc = format!("Modifies the `{}` field.", self.name);
        let visibility = &self.bitfield.input.visibility;
        let modify_method = &self.modify_method;
        let accessor_type = &self.accessor_type;
        quote! {
            #[doc = #doc]
            #[inline(always)]
            #visibility fn #modify_method(&mut self, f: impl ::core::ops::FnOnce(#accessor_type) -> #accessor_type) {
                self.#set_method(f(self.#get_method()));
            }
        }
    }
}

impl<'a> ToTokens for BitfieldCodegenContext<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let get_method = self.make_get_method();
        let with_method = self.make_with_method();
        let map_method = self.make_map_method();
        let set_method = self.make_set_method();
        let modify_method = self.make_modify_method();

        tokens.extend(quote! {
            #get_method
            #with_method
            #map_method
            #set_method
            #modify_method
        })
    }
}
