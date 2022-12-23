use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;

use crate::ast::StructAst;
use crate::types::{
    convert, BoolType, BorrowedType, NarrowIntegerType, OwnedType, UserType, UserTypeRepr,
};

fn lsb_mask(width: usize) -> usize {
    (1 << width) - 1
}

fn lsb_offset_mask(offset: usize, width: usize) -> usize {
    lsb_mask(width) << offset
}

pub(crate) struct StructDecl {
    header: StructHeader,
    fields: Vec<Field>,
}

impl StructDecl {
    pub fn from_ast(ast: StructAst) -> Self {
        StructDecl {
            header: StructHeader {
                visibility: ast.header.visibility,
                name: ast.header.name,
                repr: OwnedType::from_ident(ast.header.repr),
            },
            fields: ast
                .fields
                .into_iter()
                .map(|ast| {
                    let width = ast.width.to_string().parse().unwrap();
                    Field {
                        visibility: ast.visibility,
                        name: ast.name.to_string(),
                        name_ident: ast.name,
                        width,
                        io_type: if let Some(ident) = ast.io_type {
                            if ident.to_string() == "bool" {
                                OwnedType::Bool(BoolType { ident })
                            } else {
                                OwnedType::User(UserType {
                                    repr: match OwnedType::new_integer_span(width, ast.width.span())
                                    {
                                        OwnedType::PrimitiveInteger(t) => {
                                            UserTypeRepr::PrimitiveInteger(t)
                                        }
                                        OwnedType::NarrowInteger(t) => {
                                            UserTypeRepr::NarrowInteger(t)
                                        }
                                        _ => unreachable!(),
                                    },
                                    ident,
                                })
                            }
                        } else {
                            OwnedType::new_integer_span(width, ast.width.span())
                        },
                    }
                })
                .collect(),
        }
    }

    pub fn into_token_stream(self) -> TokenStream {
        // Collect non-field methods.
        let zero_method = self.header.make_zero_method();
        let from_repr_method = self.header.make_from_repr_method();
        let as_repr_method = self.header.make_as_repr_method();

        // Collect snippets for each field.
        let mut items_for_fields = Vec::new();
        let mut offset = 0;
        for field in self.fields {
            items_for_fields.push(field.layout(&self.header, &mut offset).into_token_stream());
        }

        // Emit the struct and impl block.
        let visibility = &self.header.visibility;
        let name = &self.header.name;
        let repr = self.header.repr.to_token_stream();
        quote! {
            #[derive(Clone, Copy, PartialEq, Eq)]
            #[repr(transparent)]
            #visibility struct #name {
                value: #repr,
            }

            #[allow(dead_code)]
            impl #name {
                #zero_method
                #from_repr_method
                #as_repr_method

                #(#items_for_fields)*
            }
        }
    }
}

#[derive(Debug)]
struct StructHeader {
    visibility: TokenStream,
    name: Ident,
    repr: OwnedType,
}

impl StructHeader {
    fn make_from_repr_method(&self) -> TokenStream {
        let method = Ident::new(
            &format!("from_{}", self.repr.to_method_name_snippet()),
            Span::call_site(),
        );
        let repr = self.repr.to_token_stream();
        quote! {
            #[inline(always)]
            pub const fn #method(value: #repr) -> Self {
                Self { value }
            }
        }
    }

    fn make_as_repr_method(&self) -> TokenStream {
        let method = Ident::new(
            &format!("as_{}", self.repr.to_method_name_snippet()),
            Span::call_site(),
        );
        let repr = self.repr.to_token_stream();
        quote! {
            #[inline(always)]
            pub const fn #method(self) -> #repr {
                self.value
            }
        }
    }

    fn make_zero_method(&self) -> TokenStream {
        let expr = match &self.repr {
            OwnedType::PrimitiveInteger(_) => quote! { 0 },
            OwnedType::NarrowInteger(NarrowIntegerType { path, .. }) => {
                quote! { <#path>::new_masked(0) }
            }
            _ => panic!(),
        };
        quote! {
            #[inline(always)]
            pub const fn zero() -> Self {
                Self { value: #expr }
            }
        }
    }
}

#[derive(Debug)]
struct Field {
    visibility: TokenStream,
    name: String,
    name_ident: Ident,
    width: usize,
    io_type: OwnedType,
}

impl Field {
    fn layout<'a>(&'a self, header: &'a StructHeader, offset: &mut usize) -> FieldCtx<'a> {
        let this_offset = *offset;
        *offset += self.width;
        FieldCtx {
            header,
            field: self,
            offset: this_offset,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct FieldCtx<'a> {
    header: &'a StructHeader,
    field: &'a Field,
    offset: usize,
}

impl FieldCtx<'_> {
    fn into_token_stream(self) -> TokenStream {
        if self.field.name.starts_with('_') {
            return TokenStream::new();
        }

        let repr_type = self.header.repr.to_borrowed();
        let work_type = self.header.repr.to_primitive();
        let io_type = self.field.io_type.to_borrowed();

        let get_method = self.make_get_method(repr_type, work_type, io_type);
        let with_method = self.make_with_method(repr_type, work_type, io_type);
        let map_method = self.make_map_method(io_type);
        let set_method = self.make_set_method(repr_type, work_type, io_type);
        let modify_method = self.make_modify_method(io_type);

        quote! {
            #get_method
            #with_method
            #map_method
            #set_method
            #modify_method
        }
    }

    fn make_get_method(
        &self,
        repr_type: BorrowedType,
        work_type: BorrowedType,
        io_type: BorrowedType,
    ) -> TokenStream {
        let shift = Literal::usize_unsuffixed(self.offset);
        let lsb_mask = Literal::usize_unsuffixed(lsb_mask(self.field.width));

        let self_value_work = convert(quote! { self.value }, repr_type, work_type);
        let result_io = convert(
            quote! { ((#self_value_work) >> #shift) & #lsb_mask },
            work_type,
            io_type,
        );

        let visibility = &self.field.visibility;
        let name = &self.field.name_ident;
        let doc = format!("Extracts the `{name}` field.");
        let io_type = self.field.io_type.to_token_stream();
        quote! {
            #[doc = #doc]
            #[inline(always)]
            #visibility const fn #name(self) -> #io_type {
                #result_io
            }
        }
    }

    fn make_with_method(
        &self,
        repr_type: BorrowedType,
        work_type: BorrowedType,
        io_type: BorrowedType,
    ) -> TokenStream {
        let offset_mask = Literal::usize_unsuffixed(lsb_offset_mask(self.offset, self.field.width));
        let shift = Literal::usize_unsuffixed(self.offset);

        let self_value_work = convert(quote! { self.value }, repr_type, work_type);
        let field_work = convert(quote! { value }, io_type, work_type);
        let result_repr = convert(
            quote! { ((#self_value_work) & !#offset_mask) | field },
            work_type,
            repr_type,
        );

        let visibility = &self.field.visibility;
        let name = Ident::new(
            &format!("with_{}", self.field.name),
            self.field.name_ident.span(),
        );
        let doc = format!(
            "Returns a new value with the `{}` field inserted.",
            self.field.name,
        );
        let io_type = io_type.to_token_stream();
        quote! {
            #[doc = #doc]
            #[inline(always)]
            #visibility const fn #name(self, value: #io_type) -> Self {
                let field = (#field_work) << #shift;

                // This is a redundant operation but it helps the compiler emit the `rlwimi`
                // instruction on PowerPC.
                #[cfg(target_arch = "powerpc")]
                let field = field & #offset_mask;

                Self { value: #result_repr }
            }
        }
    }

    fn make_map_method(&self, io_type: BorrowedType) -> TokenStream {
        let get_method = &self.field.name_ident;
        let with_method = Ident::new(
            &format!("with_{}", self.field.name),
            self.field.name_ident.span(),
        );

        let visibility = &self.field.visibility;
        let name = Ident::new(
            &format!("map_{}", self.field.name),
            self.field.name_ident.span(),
        );
        let doc = format!(
            "Returns a new value with the `{}` field mapped.",
            self.field.name,
        );
        let io_type = io_type.to_token_stream();
        quote! {
            #[doc = #doc]
            #[inline(always)]
            #visibility fn #name(self, f: impl FnOnce(#io_type) -> #io_type) -> Self {
                self.#with_method(f(self.#get_method()))
            }
        }
    }

    fn make_set_method(
        &self,
        repr_type: BorrowedType,
        work_type: BorrowedType,
        io_type: BorrowedType,
    ) -> TokenStream {
        let offset_mask = Literal::usize_unsuffixed(lsb_offset_mask(self.offset, self.field.width));
        let shift = Literal::usize_unsuffixed(self.offset);

        let self_value_work = convert(quote! { self.value }, repr_type, work_type);
        let value_work = convert(quote! { value }, io_type, work_type);
        let result_repr = convert(
            quote! { ((#self_value_work) & !#offset_mask) | field },
            work_type,
            repr_type,
        );

        let visibility = &self.field.visibility;
        let name = Ident::new(
            &format!("set_{}", self.field.name),
            self.field.name_ident.span(),
        );
        let doc = format!("Inserts the `{}` field.", self.field.name);
        let io_type = self.field.io_type.to_token_stream();
        quote! {
            #[doc = #doc]
            #[inline(always)]
            #visibility fn #name(&mut self, value: #io_type) {
                let field = (#value_work) << #shift;

                // This is a redundant operation but it helps the compiler emit the `rlwimi`
                // instruction on PowerPC.
                #[cfg(target_arch = "powerpc")]
                let field = field & #offset_mask;

                self.value = #result_repr;
            }
        }
    }

    fn make_modify_method(&self, io_type: BorrowedType) -> TokenStream {
        let get_method = &self.field.name_ident;
        let set_method = Ident::new(
            &format!("set_{}", self.field.name),
            self.field.name_ident.span(),
        );

        let visibility = &self.field.visibility;
        let name = Ident::new(
            &format!("modify_{}", self.field.name),
            self.field.name_ident.span(),
        );
        let doc = format!("Modifies the `{}` field.", self.field.name);
        let io_type = io_type.to_token_stream();
        quote! {
            #[doc = #doc]
            #[inline(always)]
            #visibility fn #name(&mut self, f: impl FnOnce(#io_type) -> #io_type) {
                self.#set_method(f(self.#get_method()));
            }
        }
    }
}
