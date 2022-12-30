use std::fmt::{self, Display, Formatter};

use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::quote;

#[derive(Clone, Debug)]
pub enum OwnedType {
    PrimitiveInteger(PrimitiveIntegerType),
    NarrowInteger(NarrowIntegerType),
    Bool(BoolType),
    User(UserType),
}

impl OwnedType {
    pub fn to_borrowed(&self) -> BorrowedType {
        match self {
            Self::PrimitiveInteger(t) => BorrowedType::PrimitiveInteger(t),
            Self::NarrowInteger(t) => BorrowedType::NarrowInteger(t),
            Self::Bool(t) => BorrowedType::Bool(t),
            Self::User(t) => BorrowedType::User(t),
        }
    }

    pub fn to_token_stream(&self) -> TokenStream {
        self.to_borrowed().to_token_stream()
    }

    pub fn to_method_name_snippet(&self) -> String {
        match self {
            Self::PrimitiveInteger(PrimitiveIntegerType { kind, .. }) => kind.as_str().to_string(),
            Self::NarrowInteger(NarrowIntegerType { bits, .. }) => format!("u{bits}"),
            _ => panic!(),
        }
    }

    pub fn new_integer_span(bits: usize, span: Span) -> Self {
        let primitive =
            PrimitiveIntegerType::new_span(PrimitiveIntegerTypeKind::for_bits(bits), span);
        if primitive.kind.bits() == bits {
            Self::PrimitiveInteger(primitive)
        } else {
            Self::NarrowInteger(NarrowIntegerType::new_span(primitive, bits, span))
        }
    }

    pub fn from_ident(ident: Ident) -> OwnedType {
        let name = ident.to_string();
        if name.starts_with('u') {
            Self::PrimitiveInteger(PrimitiveIntegerType::new_span(
                PrimitiveIntegerTypeKind::from_str(&name),
                ident.span(),
            ))
        } else if name.starts_with('U') {
            let bits: usize = name[1..].parse().unwrap();
            Self::NarrowInteger(NarrowIntegerType::new_span(
                PrimitiveIntegerType::new_span(
                    PrimitiveIntegerTypeKind::for_bits(bits),
                    ident.span(),
                ),
                bits,
                ident.span(),
            ))
        } else {
            panic!("unrecognized underlying type: {ident}")
        }
    }

    pub fn to_primitive(&self) -> BorrowedType {
        match self {
            Self::PrimitiveInteger(t) => BorrowedType::PrimitiveInteger(t),
            Self::NarrowInteger(NarrowIntegerType { repr, .. }) => {
                BorrowedType::PrimitiveInteger(repr)
            }
            _ => panic!(),
        }
    }

    pub fn bits(&self) -> usize {
        match self {
            Self::PrimitiveInteger(t) => t.kind.bits(),
            Self::NarrowInteger(t) => t.bits,
            _ => panic!(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BorrowedType<'a> {
    PrimitiveInteger(&'a PrimitiveIntegerType),
    NarrowInteger(&'a NarrowIntegerType),
    Bool(&'a BoolType),
    User(&'a UserType),
}

impl<'a> BorrowedType<'a> {
    pub fn kind(&self) -> TypeKind {
        match self {
            Self::PrimitiveInteger(t) => TypeKind::PrimitiveInteger(t.kind),
            Self::NarrowInteger(t) => TypeKind::NarrowInteger(t.kind()),
            Self::Bool(_) => TypeKind::Bool,
            Self::User(_) => TypeKind::User,
        }
    }

    pub fn to_token_stream(&self) -> TokenStream {
        match self {
            Self::PrimitiveInteger(PrimitiveIntegerType { ident, .. }) => {
                TokenStream::from_iter([TokenTree::Ident(ident.clone())])
            }
            Self::NarrowInteger(NarrowIntegerType { path, .. }) => path.clone(),
            Self::Bool(BoolType { ident }) => {
                TokenStream::from_iter([TokenTree::Ident(ident.clone())])
            }
            Self::User(UserType { ident, .. }) => {
                TokenStream::from_iter([TokenTree::Ident(ident.clone())])
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TypeKind {
    PrimitiveInteger(PrimitiveIntegerTypeKind),
    NarrowInteger(NarrowIntegerTypeKind),
    Bool,
    User,
}

impl PartialEq for TypeKind {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::PrimitiveInteger(lhs), Self::PrimitiveInteger(rhs)) => lhs == rhs,
            (Self::NarrowInteger(lhs), Self::NarrowInteger(rhs)) => lhs == rhs,
            (Self::Bool, Self::Bool) => true,
            (Self::User, Self::User) => false, // All user types are distinct.
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PrimitiveIntegerType {
    pub kind: PrimitiveIntegerTypeKind,
    pub ident: Ident,
}

impl PrimitiveIntegerType {
    fn new_span(kind: PrimitiveIntegerTypeKind, span: Span) -> Self {
        PrimitiveIntegerType {
            kind,
            ident: Ident::new(kind.as_str(), span),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrimitiveIntegerTypeKind {
    U8,
    U16,
    U32,
    U64,
    U128,
}

impl PrimitiveIntegerTypeKind {
    pub fn from_str(value: &str) -> Self {
        match value {
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" => Self::U32,
            "u64" => Self::U64,
            "u128" => Self::U128,
            x => panic!("Unsupported primitive type: {x:?}"),
        }
    }

    pub fn for_bits(bits: usize) -> Self {
        if bits <= 8 {
            Self::U8
        } else if bits <= 16 {
            Self::U16
        } else if bits <= 32 {
            Self::U32
        } else if bits <= 64 {
            Self::U64
        } else if bits <= 128 {
            Self::U128
        } else {
            panic!()
        }
    }

    pub fn bits(self) -> usize {
        match self {
            Self::U8 => 8,
            Self::U16 => 16,
            Self::U32 => 32,
            Self::U64 => 64,
            Self::U128 => 128,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::U128 => "u128",
        }
    }
}

impl Display for PrimitiveIntegerTypeKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Clone, Debug)]
pub struct NarrowIntegerType {
    pub repr: PrimitiveIntegerType,
    pub bits: usize,
    pub path: TokenStream,
}

impl NarrowIntegerType {
    pub fn new_span(repr: PrimitiveIntegerType, bits: usize, span: Span) -> Self {
        let ident = Ident::new(&format!("U{bits}"), span);
        Self {
            repr,
            bits,
            path: quote! { ::mvbitfield::narrow_integer::#ident },
        }
    }

    pub fn kind(&self) -> NarrowIntegerTypeKind {
        NarrowIntegerTypeKind {
            repr: self.repr.kind,
            bits: self.bits,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NarrowIntegerTypeKind {
    pub repr: PrimitiveIntegerTypeKind,
    pub bits: usize,
}

#[derive(Clone, Debug)]
pub struct BoolType {
    pub ident: Ident,
}

#[derive(Clone, Debug)]
pub struct UserType {
    pub repr: UserTypeRepr,
    pub ident: Ident,
}

#[derive(Clone, Debug)]
pub enum UserTypeRepr {
    PrimitiveInteger(PrimitiveIntegerType),
    NarrowInteger(NarrowIntegerType),
}

impl UserTypeRepr {
    pub fn bits(&self) -> usize {
        match self {
            Self::PrimitiveInteger(PrimitiveIntegerType { kind, .. }) => kind.bits(),
            Self::NarrowInteger(NarrowIntegerType { bits, .. }) => *bits,
        }
    }

    fn to_borrowed_type(&self) -> BorrowedType {
        match self {
            UserTypeRepr::PrimitiveInteger(t) => BorrowedType::PrimitiveInteger(t),
            UserTypeRepr::NarrowInteger(t) => BorrowedType::NarrowInteger(t),
        }
    }
}

pub fn convert(expr: TokenStream, from: BorrowedType, to: BorrowedType) -> TokenStream {
    match (from, to) {
        // No conversion needed if the types are the same.
        _ if from.kind() == to.kind() => expr,

        // Conversions between different primitive integer types.
        (
            BorrowedType::PrimitiveInteger(_),
            BorrowedType::PrimitiveInteger(PrimitiveIntegerType { ident, .. }),
        ) => quote! { (#expr) as #ident },

        // Conversions away from primitive integers to narrow integers, bools, and user-defined
        // types.
        (
            BorrowedType::PrimitiveInteger(_),
            BorrowedType::NarrowInteger(NarrowIntegerType { repr, path, .. }),
        ) => {
            // Recurse to produce the narrow integer's repr, then construct the narrow integer.
            let expr = convert(expr, from, BorrowedType::PrimitiveInteger(repr));
            quote! { <#path>::new_masked(#expr) }
        }
        (BorrowedType::PrimitiveInteger(_), BorrowedType::Bool(_)) => quote! { (#expr) != 0 },
        (BorrowedType::PrimitiveInteger(_), BorrowedType::User(UserType { repr, ident })) => {
            // Recurse to produce the user type's repr, then construct the user type.
            let expr = convert(expr, from, repr.to_borrowed_type());
            let method = Ident::new(&format!("from_u{}", repr.bits()), ident.span());
            quote! { <#ident>::#method(#expr) }
        }

        // Conversions to primitive integers from narrow integers, bools, and user-defined types.
        (BorrowedType::User(UserType { repr, ident }), BorrowedType::PrimitiveInteger(_)) => {
            // Convert from the user type to its repr and recurse.
            let method = Ident::new(&format!("as_u{}", repr.bits()), ident.span());
            convert(
                quote! { <#ident>::#method(#expr) },
                repr.to_borrowed_type(),
                to,
            )
        }
        (
            BorrowedType::NarrowInteger(NarrowIntegerType { repr, path, .. }),
            BorrowedType::PrimitiveInteger(_),
        ) => {
            // Convert from the narrow integer type to its repr and recurse.
            let method = Ident::new(&format!("as_u{}", repr.kind.bits()), repr.ident.span());
            convert(
                quote! { <#path>::#method(#expr) },
                BorrowedType::PrimitiveInteger(repr),
                to,
            )
        }
        (
            BorrowedType::Bool(_),
            BorrowedType::PrimitiveInteger(PrimitiveIntegerType { ident, .. }),
        ) => quote! { (#expr) as #ident },

        // Composite conversions.
        (BorrowedType::NarrowInteger(NarrowIntegerType { repr, .. }), BorrowedType::Bool(_)) => {
            let mid = BorrowedType::PrimitiveInteger(repr);
            convert(convert(expr, from, mid), mid, to)
        }
        (BorrowedType::Bool(_), BorrowedType::NarrowInteger(NarrowIntegerType { repr, .. })) => {
            let mid = BorrowedType::PrimitiveInteger(repr);
            convert(convert(expr, from, mid), mid, to)
        }

        _ => unimplemented!("convert from {:?} to {:?}", from.kind(), to.kind()),
    }
}
