use core::fmt::{self, Display};
use core::ops::Add;

use super::prelude::*;
use crate::no_std::prelude::*;

#[derive(Any, Debug, PartialEq)]
#[rune_derive(ADD, STRING_DEBUG, STRING_DISPLAY)]
// To test the manual handler
#[rune_derive(INDEX_GET = |it: Self, _: usize| it.0)]
#[rune_functions(Self::new)]
struct Struct(usize);

impl Struct {
    #[rune::function(path = Self::new)]
    fn new(it: usize) -> Self {
        Self(it)
    }
}

impl Add for Struct {
    type Output = Self;

    fn add(mut self, other: Self) -> Self {
        self.0 += other.0;
        self
    }
}

impl Display for Struct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[test]
fn rune_derive() -> Result<()> {
    let mut m = Module::new();
    m.ty::<Struct>()?;
    assert_eq!(
        rune_n! {
            &m,
            (),
            Struct => pub fn main() {Struct::new(1) + Struct::new(2)}
        },
        Struct(3)
    );

    assert_eq!(
        rune_n! {
            &m,
            (),
            String => pub fn main() {format!("{}", Struct::new(1))}
        },
        "1"
    );

    assert_eq!(
        rune_n! {
            &m,
            (),
            String => pub fn main() {format!("{:?}", Struct::new(1))}
        },
        "Struct(1)"
    );

    assert_eq!(
        rune_n! {
            &m,
            (),
            usize => pub fn main() {Struct::new(1)[0]}
        },
        1
    );
    Ok(())
}
