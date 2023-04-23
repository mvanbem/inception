use proc_macro2::Span;
use syn::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackDir {
    LsbFirst,
    MsbFirst,
}

pub trait Bitfield {
    fn width(&self) -> Result<Option<u8>>;
    fn error_span(&self) -> Span;
}

impl Bitfield for crate::ast::Bitfield {
    fn width(&self) -> Result<Option<u8>> {
        self.width()
    }

    fn error_span(&self) -> Span {
        self.name_span()
    }
}

struct KnownWidth<T> {
    bitfield: T,
    width: usize,
}

pub struct Packed<T> {
    pub bitfield: T,
    pub offset: usize,
    pub width: usize,
    pub width_span: Span,
}

pub fn pack<T: Bitfield, I: Iterator<Item = T> + ExactSizeIterator>(
    pack_dir: Option<PackDir>,
    struct_error_span: Span,
    struct_width: usize,
    bitfields: impl IntoIterator<IntoIter = I>,
) -> Result<Vec<Packed<T>>> {
    let bitfields = bitfields.into_iter();
    let pack_dir = match pack_dir {
        Some(pack_dir) => pack_dir,
        None if bitfields.len() < 2 => PackDir::LsbFirst, // Doesn't matter, just pick one.
        None => {
            return Err(Error::new(
                struct_error_span,
                "a packing direction attribute is required (`#[lsb_first]` or `#[msb_first]`)",
            ))
        }
    };

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
                dst.push(KnownWidth {
                    bitfield,
                    width: width as usize,
                });
            }
            None => {
                if flexible_bitfield.is_some() {
                    return Err(Error::new(
                        bitfield.error_span(),
                        "only up to one flexible bitfield is permitted",
                    ));
                } else {
                    flexible_bitfield = Some(bitfield);
                }
            }
        }
    }

    // Compute available bits after considering all sized bitfields.
    let mut available = struct_width;
    for &KnownWidth {
        ref bitfield,
        width,
    } in bitfields_before_flexible
        .iter()
        .chain(bitfields_after_flexible.iter())
    {
        if let Some(new_available) = available.checked_sub(width) {
            available = new_available;
        } else {
            return Err(Error::new(
                bitfield.error_span(),
                format!("bitfield overflows containing struct; {available} bit(s) available"),
            ));
        }
    }

    // Size the flexible bitfield, if present.
    let flexible_bitfield = match flexible_bitfield {
        Some(bitfield) if available > 0 => Some(KnownWidth {
            bitfield,
            width: available,
        }),
        Some(bitfield) => {
            return Err(Error::new(
                bitfield.error_span(),
                format!("no bits available for flexible bitfield"),
            ))
        }
        None if available == 0 => None,
        None => {
            return Err(Error::new(
                struct_error_span,
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
        PackDir::MsbFirst => struct_width,
    };
    let mut packed = Vec::new();
    for KnownWidth { bitfield, width } in bitfields_before_flexible
        .into_iter()
        .chain(flexible_bitfield.into_iter())
        .chain(bitfields_after_flexible.into_iter())
    {
        let width_span = bitfield.error_span();
        packed.push(Packed {
            bitfield,
            offset: match pack_dir {
                PackDir::LsbFirst => {
                    let this_lsb_offset = lsb_offset;
                    lsb_offset += width;
                    this_lsb_offset
                }
                PackDir::MsbFirst => {
                    lsb_offset -= width;
                    lsb_offset
                }
            },
            width,
            width_span,
        });
    }
    Ok(packed)
}
