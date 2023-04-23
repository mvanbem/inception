use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    braced, parenthesized, token, Attribute, Ident, LitInt, Path, Result, Token, Type, Visibility,
};

pub struct Input {
    _paren_token: token::Paren,
    pub crate_path: Path,
    _comma_token: Token![,],
    pub structs: Vec<Struct>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(Input {
            _paren_token: parenthesized!(content in input),
            crate_path: content.parse()?,
            _comma_token: content.parse()?,
            structs: {
                let mut structs = Vec::new();
                while !content.is_empty() {
                    structs.push(content.parse()?);
                }
                structs
            },
        })
    }
}

pub struct Struct {
    pub attrs: Vec<Attribute>,
    pub visibility: Visibility,
    _struct_token: Token![struct],
    pub name: Ident,
    _colon_token: Token![:],
    pub width: LitInt,
    _brace_token: token::Brace,
    pub bitfields: Punctuated<Bitfield, Token![,]>,
}

impl Parse for Struct {
    fn parse(input: ParseStream) -> Result<Self> {
        let body;
        Ok(Self {
            attrs: input.call(Attribute::parse_outer)?,
            visibility: Visibility::parse(input)?,
            _struct_token: input.parse()?,
            name: input.parse()?,
            _colon_token: input.parse()?,
            width: input.parse()?,
            _brace_token: braced!(body in input),
            bitfields: body.parse_terminated(Bitfield::parse, Token![,])?,
        })
    }
}

pub struct Bitfield {
    pub attrs: Vec<Attribute>,
    pub visibility: Visibility,
    variant: BitfieldVariant,
}

enum BitfieldVariant {
    Regular {
        name: BitfieldName,
        _colon_token: Token![:],
        width: BitfieldWidth,
        accessor_type: AccessorType,
    },
    DotDot {
        dot_dot_token: Token![..],
    },
}

impl Bitfield {
    pub fn name_to_string(&self) -> String {
        match &self.variant {
            BitfieldVariant::Regular {
                name: BitfieldName::Ident(ident),
                ..
            } => ident.to_string(),
            _ => "_".to_string(),
        }
    }

    pub fn name_span(&self) -> Span {
        match &self.variant {
            BitfieldVariant::Regular { name, .. } => match name {
                BitfieldName::Ident(ident) => ident.span(),
                BitfieldName::Placeholder(underscore) => underscore.span(),
            },
            BitfieldVariant::DotDot { dot_dot_token } => dot_dot_token.span(),
        }
    }

    pub fn width(&self) -> Result<Option<u8>> {
        match &self.variant {
            BitfieldVariant::Regular {
                width: BitfieldWidth::LitInt(lit_int),
                ..
            } => Ok(Some(lit_int.base10_parse()?)),
            _ => Ok(None),
        }
    }

    pub fn accessor_type(&self) -> AccessorType {
        match &self.variant {
            BitfieldVariant::Regular { accessor_type, .. } => accessor_type.clone(),
            BitfieldVariant::DotDot { .. } => AccessorType::Default,
        }
    }
}

impl Parse for Bitfield {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            attrs: input.call(Attribute::parse_outer)?,
            visibility: input.parse()?,
            variant: {
                let lookahead = input.lookahead1();
                if lookahead.peek(Token![..]) {
                    BitfieldVariant::DotDot {
                        dot_dot_token: input.parse()?,
                    }
                } else if lookahead.peek(Ident) | lookahead.peek(Token![_]) {
                    BitfieldVariant::Regular {
                        name: input.parse()?,
                        _colon_token: input.parse()?,
                        width: input.parse()?,
                        accessor_type: input.parse()?,
                    }
                } else {
                    return Err(lookahead.error());
                }
            },
        })
    }
}

pub enum BitfieldName {
    Ident(Ident),
    Placeholder(Token![_]),
}

impl Parse for BitfieldName {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Ident) {
            input.parse().map(Self::Ident)
        } else if lookahead.peek(Token![_]) {
            input.parse().map(Self::Placeholder)
        } else {
            Err(lookahead.error())
        }
    }
}

pub enum BitfieldWidth {
    LitInt(LitInt),
    Placeholder(Token![_]),
}

impl Parse for BitfieldWidth {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(LitInt) {
            input.parse().map(Self::LitInt)
        } else if lookahead.peek(Token![_]) {
            input.parse().map(Self::Placeholder)
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Clone)]
pub enum AccessorType {
    Overridden { _as_token: Token![as], type_: Type },
    Default,
}

impl Parse for AccessorType {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![as]) {
            Ok(AccessorType::Overridden {
                _as_token: input.parse()?,
                type_: input.parse()?,
            })
        } else {
            Ok(AccessorType::Default)
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;

    #[test]
    fn struct_empty() {
        let input = quote! { struct Foo: 32 {} };
        let Struct {
            attrs,
            visibility,
            name,
            width,
            bitfields: fields,
            ..
        } = syn::parse2(input).unwrap();
        assert!(attrs.is_empty());
        assert_eq!(quote! { #visibility }.to_string(), "");
        assert_eq!(quote! { #name }.to_string(), "Foo");
        assert_eq!(width.base10_digits(), "32");
        assert_eq!(fields.len(), 0);
    }

    #[test]
    fn struct_everything() {
        let input = quote! {
            /// this has a doc comment
            pub(crate) struct Bar: 5 {
                field: 1
            }
        };
        let Struct {
            attrs,
            visibility,
            name,
            width,
            bitfields: fields,
            ..
        } = syn::parse2(input).unwrap();
        assert_eq!(attrs.len(), 1);
        let attr = &attrs[0];
        assert_eq!(
            quote! { #attr }.to_string(),
            "# [doc = r\" this has a doc comment\"]",
        );
        assert_eq!(quote! { #visibility }.to_string(), "pub (crate)");
        assert_eq!(name.to_string(), "Bar");
        assert_eq!(width.base10_digits(), "5");
        assert_eq!(fields.len(), 1);
    }

    #[test]
    fn field_default() {
        let input = quote! { my_field: 5 };
        let Bitfield {
            attrs,
            visibility: Visibility::Inherited,
            variant: BitfieldVariant::Regular {
                name: BitfieldName::Ident(name),
                width: BitfieldWidth::LitInt(width),
                accessor_type: AccessorType::Default,
                ..
            },
        } = syn::parse2(input).unwrap() else { panic!() };
        assert!(attrs.is_empty());
        assert_eq!(name.to_string(), "my_field");
        assert_eq!(width.to_string(), "5");
    }

    #[test]
    fn field_everything() {
        let input = quote! { pub(crate) my_field: 5 as path::to::Bar };
        let Bitfield {
            attrs,
            visibility,
            variant: BitfieldVariant::Regular {
                name: BitfieldName::Ident(name),
                width: BitfieldWidth::LitInt(width),
                accessor_type:
                    AccessorType::Overridden {
                        type_: accessor_type,
                        ..
                    },
                ..
            },
        } = syn::parse2(input).unwrap() else { panic!() };
        assert!(attrs.is_empty());
        assert_eq!(quote! { #visibility }.to_string(), "pub (crate)");
        assert_eq!(name.to_string(), "my_field");
        assert_eq!(width.to_string(), "5");
        assert_eq!(quote! { #accessor_type }.to_string(), "path :: to :: Bar");
    }

    #[test]
    fn field_flexible() {
        let input = quote! { .. };
        assert!(matches!(
            syn::parse2(input).unwrap(),
            Bitfield {
                variant: BitfieldVariant::DotDot { .. },
                ..
            },
        ));
    }
}
