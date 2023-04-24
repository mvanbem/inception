# Syntax

This page uses [notation from The Rust Reference](https://doc.rust-lang.org/reference/notation.html)
for syntax grammar snippets. Tokens and production rules from Rust link to their definitions in The
Rust Reference. New production rules are unlinked and are all defined on this page.

## Invocation

> **Syntax**
>
> _Input_ :
>
> > _Struct_<sup>\*</sup>

An `mvbitfield!` macro invocation must receive one _Input_ declaring zero or more structs.

## Bitfield Structs

> **Syntax**
>
> _Struct_ :
>
> > [_OuterAttribute_][RefAttr]<sup>\*</sup> [_Visibility_][RefVis]<sup>?</sup> `struct`
>   [IDENTIFIER][RefIdent] `:` [INTEGER_LITERAL][RefLitInt] `{` _StructElements_<sup>?</sup> `}`
>
> _StructElements_ :
>
> >  _StructElement_ (`,` _Bitfield_)<sup>\*</sup> `,`<sup>?</sup>

Each _Struct_ declares a bitfield struct and specifies its attributes, visibility, name,
width, and bitfields.

A bitfield struct's width must match the sum of the widths of its bitfields. The `..`
_StructElement_ may appear zero or one times in a bitfield struct; if present, it declares a
reserved bitfield with the unique positive width that satisfies the overall width constraint.

## Struct Attributes

A struct attribute with any of the paths below is parsed and interpreted as noted; any other struct
attribute is applied to the generated bitfield struct.

Packing order paths:

* `lsb_first`: Must not have a value. Sets the struct packing order to LSB-first.
* `msb_first`: Must not have a value. Sets the struct packing order to MSB-first.

Packing order attributes may not repeat and are mutually exclusive.

## Bitfields

> **Syntax**
>
> _Bitfield_ :
>
> > [_OuterAttribute_][RefAttr]<sup>\*</sup> [_Visibility_][RefVis]<sup>?</sup>
> > ([IDENTIFIER][RefIdent] | `_`) `:` ([INTEGER_LITERAL][RefLitInt] | `_`) (`as` [_Type_][RefType]
>   )<sup>?</sup>
>
> > | `..`

Each _Bitfield_ declares a bitfield and specifies its attributes, visibility, name, width, and
accessor type override.

The name may be an identifier or the placeholder `_`.

The width may be an integer literal or the placeholder `_`.

The alternate `..` form is equivalent to `_: _`; that is, no attributes, private visibility,
placeholder name, placeholder width, and no accessor type override.

## Bitfield Attributes

A bitfield attribute with any of the paths below is parsed and interpreted as noted; other paths are
reserved and will raise a compile error.


[RefAttr]: https://doc.rust-lang.org/reference/attributes.html
[RefIdent]: https://doc.rust-lang.org/reference/identifiers.html
[RefLitInt]: https://doc.rust-lang.org/reference/tokens.html#integer-literals
[RefType]: https://doc.rust-lang.org/reference/types.html#type-expressions
[RefVis]: https://doc.rust-lang.org/reference/visibility-and-privacy.html
