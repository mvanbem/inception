use proc_macro2::{Literal, Span, TokenStream};
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{parse_quote, Error, Path, Result, TypePath};

use crate::ast::{self, AccessorType, Bitfield, Input};
use crate::pack::{pack, PackDir, Packed};

struct Config {
    crate_path: Path,
}

struct TypeInfo {
    underlying_type: TypePath,
    primitive_type: TypePath,
}

impl Config {
    /// Names the underlying and primitive types for a bitfield of the given width.
    ///
    /// This will match the associated types on `mvbitfield::Bitfield` and `bitint::BitUint`, but
    /// are resolved before the macro's output.
    fn type_info_for_width(&self, width: usize, span: Span) -> Result<TypeInfo> {
        if !(1..=128).contains(&width) {
            return Err(Error::new(
                span,
                "widths must be at least 1 and at most 128",
            ));
        }

        let primitive_widths = [8, 16, 32, 64, 128];
        match primitive_widths.binary_search(&width) {
            Ok(_) => {
                let name = format_ident!("u{width}", span = span);
                let primitive_type: TypePath = parse_quote! { #name };
                Ok(TypeInfo {
                    underlying_type: primitive_type.clone(),
                    primitive_type,
                })
            }
            Err(index) => {
                let crate_path = &self.crate_path;
                let underlying_name = format_ident!("U{width}", span = span);
                let underlying_type = parse_quote! { #crate_path::bitint::#underlying_name };

                let primitive_width = primitive_widths[index];
                let primitive_name = format_ident!("u{}", primitive_width, span = span);
                let primitive_type = parse_quote! { #primitive_name };

                Ok(TypeInfo {
                    underlying_type,
                    primitive_type,
                })
            }
        }
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
    let TypeInfo {
        underlying_type: struct_underlying_type,
        primitive_type: struct_primitive_type,
    } = cfg.type_info_for_width(struct_width, input.width.span())?;

    let crate_path = &cfg.crate_path;
    let bit_uint: Path = parse_quote! { #crate_path::bitint::BitUint };

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
            value: #struct_underlying_type,
        }

        #[allow(dead_code)]
        impl #name {
            #(#bitfield_items)*
        }

        impl ::core::convert::From<#struct_underlying_type> for #name {
            fn from(value: #struct_underlying_type) -> Self {
                Self { value }
            }
        }

        impl ::core::convert::From<#name> for #struct_underlying_type {
            fn from(value: #name) -> Self {
                value.value
            }
        }

        impl #crate_path::Bitfield for #name {
            type Underlying = #struct_underlying_type;

            const ZERO: Self = Self { value: #bit_uint::ZERO };

            fn from_underlying(value: #struct_underlying_type) -> Self {
                Self { value }
            }

            fn to_underlying(self) -> #struct_underlying_type {
                self.value
            }
        }
    })
}

fn generate_accessors(
    cfg: &Config,
    struct_primitive_type: &TypePath,
    bitfield: Packed<Bitfield>,
) -> Result<Option<TokenStream>> {
    // Reserved fields do not generate any code.
    let name = bitfield.bitfield.name_to_string();
    if name.starts_with('_') {
        return Ok(None);
    }

    let crate_path = &cfg.crate_path;
    let bitfield_trait: Path = parse_quote! { #crate_path::Bitfield };

    let visibility = &bitfield.bitfield.visibility;
    let name = bitfield.bitfield.name_to_string();
    let name_span = bitfield.bitfield.name_span();
    let TypeInfo {
        underlying_type: accessor_underlying_type,
        primitive_type: accessor_primitive_type,
    } = cfg.type_info_for_width(bitfield.width, bitfield.width_span)?;
    let accessor_type = match bitfield.bitfield.accessor_type() {
        AccessorType::Overridden { type_, .. } => type_,
        AccessorType::Default => accessor_underlying_type.clone().into(),
    };

    let shift = Literal::usize_unsuffixed(bitfield.offset);
    // let lsb_mask = Literal::usize_unsuffixed(lsb_mask(bitfield.width));
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
                #bitfield_trait::new_masked(
                    (#bitfield_trait::to_primitive(self) >> #shift) as #accessor_primitive_type,
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
                let struct_value = #bitfield_trait::to_primitive(self);
                let bitfield_value = #bitfield_trait::to_primitive(value) as #struct_primitive_type;
                // // This is a redundant operation but it helps the compiler emit the `rlwimi`
                // // instruction on PowerPC.
                // #[cfg(target_arch = "powerpc")]
                // let bitfield_value = bitfield_value & offset_mask;

                let new_value = (struct_value & !#offset_mask) | (bitfield_value << #shift);
                // SAFETY: Both operands have only in-range bits set, so the result will, too.
                unsafe { #bitfield_trait::new_unchecked(new_value) }
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
