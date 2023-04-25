use bitint::prelude::*;

fn main() {
    let _ = lit!();
    let _ = lit!(multiple idents);
    let _ = lit!(0u8 0);
    let _ = lit!(0u8 some_ident);
    let _ = lit!(0);
    let _ = lit!(0q);
    let _ = lit!(0uq);
    let _ = lit!(0u0);
    let _ = lit!(0u129);
}
