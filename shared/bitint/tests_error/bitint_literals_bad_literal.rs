use bitint::prelude::*;

#[bitint_literals]
fn with_attr() {
    let _ = 0u8; // OK
    let _ = 0; // OK
    let _ = 0q;
    let _ = 0_Uq;
    let _ = 0_U0;
    let _ = 0_U129;
}

fn main() {
    let _ = bitint!();
    let _ = bitint!(0u8); // Rejected
    let _ = bitint!(multiple idents);
    let _ = bitint!(0_U8 0);
    let _ = bitint!(0_U8 some_ident);
    let _ = bitint!(0); // Rejected
    let _ = bitint!(0q);
    let _ = bitint!(0_Uq);
    let _ = bitint!(0_U0);
    let _ = bitint!(0_U129);
}
