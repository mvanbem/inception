use proc_macro2::{Literal, Span, TokenStream};
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{parse_quote, Error, Path, Result, Type, TypePath};

use crate::ast::{self, AccessorType, Input};
use crate::pack::{pack, PackDir, PackedBitfield};

struct Config {
    crate_path: Path,
}

struct BitintTypeInfo {
    bitint_type: TypePath,
    primitive_type: TypePath,
}

impl BitintTypeInfo {
    fn with_accessor_type(self, accessor_type: AccessorType) -> AccessorTypeInfo {
        match accessor_type {
            AccessorType::Overridden { type_, .. } => match type_ {
                type_ => AccessorTypeInfo {
                    accessor_type: type_,
                    primitive_type: self.primitive_type,
                },
            },
            AccessorType::Default => AccessorTypeInfo {
                accessor_type: self.bitint_type.into(),
                primitive_type: self.primitive_type,
            },
        }
    }
}

struct AccessorTypeInfo {
    accessor_type: Type,
    primitive_type: TypePath,
}

impl Config {
    /// Names the bitint and primitive types for the given width.
    ///
    /// This will match the associated types on `mvbitfield::Accessor`, but are
    /// resolved before the macro's output to provide clearer rustdoc and editor
    /// metadata.
    fn type_info_for_width(&self, width: usize, span: Span) -> Result<BitintTypeInfo> {
        if !(1..=128).contains(&width) {
            return Err(Error::new(
                span,
                "widths must be at least 1 and at most 128",
            ));
        }

        let crate_path = &self.crate_path;
        let bitint_name = format_ident!("U{width}", span = span);
        let bitint_type = parse_quote! { #crate_path::bitint::types::#bitint_name };

        let primitive_width = width.next_power_of_two().max(8);
        let primitive_name = format_ident!("u{}", primitive_width, span = span);
        let primitive_type = parse_quote! { #primitive_name };

        Ok(BitintTypeInfo {
            bitint_type,
            primitive_type,
        })
    }
}

pub fn bitfield_impl(input: Input) -> TokenStream {
    let cfg = Config {
        crate_path: input.crate_path,
    };
    let results: Vec<_> = input
        .structs
        .into_iter()
        .map(|struct_| generate_struct(&cfg, struct_))
        .collect();
    quote! { #(#results)* }
}

fn generate_struct(cfg: &Config, input: ast::Struct) -> TokenStream {
    let cloned_name = input.name.clone();
    match generate_struct_impl(cfg, input) {
        Ok(result) => result,
        Err(e) => {
            let compile_error = e.into_compile_error();
            quote! {
                #compile_error
                struct #cloned_name {}
            }
        }
    }
}

fn generate_struct_impl(cfg: &Config, input: ast::Struct) -> Result<TokenStream> {
    let mut pack_dir = None;
    let mut other_attrs = Vec::new();

    for attr in input.attrs {
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

    let name = input.name;
    let struct_width = input.width.base10_parse()?;
    let BitintTypeInfo {
        primitive_type: struct_primitive_type,
        bitint_type: struct_bitint_type,
    } = cfg.type_info_for_width(struct_width, input.width.span())?;

    let crate_path = &cfg.crate_path;

    // Collect bitfield items.
    let mut bitfield_items = Vec::new();
    for bitfield in pack(pack_dir, name.span(), struct_width, input.bitfields)? {
        match generate_accessors(cfg, &struct_primitive_type, bitfield) {
            Ok(Some(tokens)) => bitfield_items.push(tokens),
            Ok(None) => (),
            Err(e) => bitfield_items.push(e.into_compile_error()),
        }
    }

    // Emit the struct and impl block.
    let visibility = input.visibility;
    Ok(quote! {
        #[derive(
            ::core::clone::Clone,
            ::core::marker::Copy,
            ::core::fmt::Debug,
        )]
        #[repr(transparent)]
        #(#other_attrs)*
        #visibility struct #name {
            value: #struct_bitint_type,
        }

        #[allow(dead_code)]
        impl #name {
            #(#bitfield_items)*
        }

        impl ::core::convert::From<#struct_bitint_type> for #name {
            fn from(value: #struct_bitint_type) -> Self {
                Self::from_bitint(value)
            }
        }

        impl ::core::convert::From<#name> for #struct_bitint_type {
            fn from(value: #name) -> Self {
                value.to_bitint()
            }
        }

        impl #crate_path::Bitfield for #name {
            type Bitint = #struct_bitint_type;

            const ZERO: Self = Self { value: #crate_path::bitint::UBitint::ZERO };

            fn from_bitint(value: #struct_bitint_type) -> Self {
                Self { value }
            }

            fn to_bitint(self) -> #struct_bitint_type {
                self.value
            }
        }
    })
}

fn generate_accessors(
    cfg: &Config,
    struct_primitive_type: &TypePath,
    bitfield: PackedBitfield,
) -> Result<Option<TokenStream>> {
    // Reserved fields do not generate any code.
    let name = bitfield.bitfield.name_to_string();
    if name.starts_with('_') {
        return Ok(None);
    }

    let crate_path = &cfg.crate_path;
    let accessor_trait: Path = parse_quote! { #crate_path::Accessor };

    let visibility = &bitfield.bitfield.visibility;
    let name = bitfield.bitfield.name_to_string();
    let name_span = bitfield.bitfield.name_span();
    let AccessorTypeInfo {
        accessor_type,
        primitive_type: accessor_primitive_type,
    } = cfg
        .type_info_for_width(bitfield.width, bitfield.width_span)?
        .with_accessor_type(bitfield.bitfield.accessor_type());

    let shift = Literal::usize_unsuffixed(bitfield.offset);
    let offset_mask = {
        let mut literal = Literal::u128_unsuffixed(
            if bitfield.width == 128 {
                u128::MAX
            } else {
                (1 << bitfield.width) - 1
            } << bitfield.offset,
        );
        literal.set_span(bitfield.width_span);
        literal
    };

    let get_method_name = format_ident!("{}", &name, span = name_span);
    let with_method_name = format_ident!("with_{}", &name, span = name_span);
    let map_method_name = format_ident!("map_{}", &name, span = name_span);
    let replace_method_name = format_ident!("replace_{}", &name, span = name_span);
    let set_method_name = format_ident!("set_{}", &name, span = name_span);
    let update_method_name = format_ident!("update_{}", &name, span = name_span);

    let get_method = {
        let doc = format!("Extracts the `{}` bitfield.", name);
        quote! {
            #[doc = #doc]
            #[inline(always)]
            #[must_use]
            #visibility fn #get_method_name(self) -> #accessor_type {
                #accessor_trait::from_primitive_masked(
                    (#accessor_trait::to_primitive(self) >> #shift) as #accessor_primitive_type,
                )
            }
        }
    };

    let with_method = {
        let doc = format!("Creates a new value with the given `{}` bitfield.", name);
        quote! {
            #[doc = #doc]
            #[inline(always)]
            #[must_use]
            #visibility fn #with_method_name(self, value: #accessor_type) -> Self {
                let struct_value = #accessor_trait::to_primitive(self);
                let bitfield_value = #accessor_trait::to_primitive(value) as #struct_primitive_type;
                // // This is a redundant operation but it helps the compiler emit the `rlwimi`
                // // instruction on PowerPC.
                // #[cfg(target_arch = "powerpc")]
                // let bitfield_value = bitfield_value & offset_mask;

                let new_value = (struct_value & !#offset_mask) | (bitfield_value << #shift);
                // SAFETY: Both operands have only in-range bits set, so the result will, too.
                unsafe { #accessor_trait::from_primitive_unchecked(new_value) }
            }
        }
    };

    let map_method: TokenStream = {
        let doc = format!(
            "Creates a new value by mapping the `{}` bitfield to a new one.",
            name,
        );
        quote! {
            #[doc = #doc]
            #[inline(always)]
            #[must_use]
            #visibility fn #map_method_name(
                self,
                f: impl ::core::ops::FnOnce(#accessor_type) -> #accessor_type,
            ) -> Self {
                self.#with_method_name(f(self.#get_method_name()))
            }
        }
    };

    let set_method: TokenStream = {
        let doc = format!("Sets the `{}` bitfield.", name);
        quote! {
            #[doc = #doc]
            #[inline(always)]
            #visibility fn #set_method_name(&mut self, value: #accessor_type) {
                *self = self.#with_method_name(value);
            }
        }
    };

    let replace_method = {
        let doc = format!(
            "Replaces the `{}` bitfield and returns the old value.",
            name,
        );
        quote! {
            #[doc = #doc]
            #[inline(always)]
            #visibility fn #replace_method_name(
                &mut self,
                value: #accessor_type,
            ) -> #accessor_type {
                let old_value = self.#get_method_name();
                self.#set_method_name(value);
                old_value
            }
        }
    };

    let update_method = {
        let doc = format!(
            "Updates the `{}` bitfield using a function and returns the old value.",
            name
        );
        quote! {
            #[doc = #doc]
            #[inline(always)]
            #visibility fn #update_method_name(
                &mut self,
                f: impl ::core::ops::FnOnce(#accessor_type) -> #accessor_type,
            ) -> #accessor_type {
                self.#replace_method_name(f(self.#get_method_name()))
            }
        }
    };

    Ok(Some(quote! {
        #get_method
        #with_method
        #map_method
        #set_method
        #replace_method
        #update_method
    }))
}
