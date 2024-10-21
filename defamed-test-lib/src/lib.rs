//! Test lib for defamed
//!
#![allow(clippy::let_unit_value)]

use std::borrow::Cow;

mod tests;

/// This function is public, so it can be used by other crates as well as internally.
///
/// ```
/// let add_default = defamed_test_lib::some_root_function!("base");
/// let add_known = defamed_test_lib::some_root_function!("base", concat = Some(" concat"));
///
/// assert_eq!(add_default, "base");
/// assert_eq!(add_known, "base concat");
/// ```
#[defamed::defamed(crate)]
pub fn some_root_function<'a>(base: &'a str, #[def] concat: Option<&str>) -> Cow<'a, str> {
    let _ = complex_function!(1, 2);
    let _ = complex_function!(lhs = 2, rhs = 2);

    match concat {
        Some(c) => Cow::Owned(base.to_owned() + c),
        None => Cow::Borrowed(base),
    }
}

/// It is possible to annotate functions without any parameters,
/// but nothing useful is generated.
#[defamed::defamed(crate)]
fn no_params() {}

pub mod inner {

    /// Mask the base value with a mask and shift the result right by `r_shift` bits.
    /// Returns `true` if the LSB of the result is set, `false` otherwise.
    #[defamed::defamed(inner)]
    pub fn nested_inner_function(base: u8, mask: u8, #[def] r_shift: u8) -> bool {
        let inter = base & mask;
        let shifted = inter >> r_shift;

        let _unit = super::no_params!();

        shifted & 1 != 0
    }
}

pub const DEFAULT_SLICE: &[u8] = &[];

/// Some struct definition
#[defamed::defamed(crate)]
pub struct DefaultStruct<'a> {
    pub index: usize,
    #[def]
    pub offset: usize,
    #[def(const DEFAULT_SLICE)]
    pub inner: &'a [u8],
}

impl<'a> DefaultStruct<'a> {
    /// Get the value at the index + offset
    pub fn value_at(&'a self) -> Option<u8> {
        self.inner.get(self.index + self.offset).cloned()
    }
}

/// Some struct tuple definition
#[defamed::defamed(crate)]
#[derive(Clone, Debug, PartialEq)]
pub struct DefaultTupleStruct(pub usize, #[def] pub usize, #[def('a')] pub char);

#[defamed::defamed]
fn complex_function(
    lhs: i32,
    rhs: i32,
    // literals can be used as default values
    #[def(true)] add: bool,
    // if no default value is provided, the type must implement Default
    #[def] divide_result_by: Option<i32>,
) -> i32 {
    let intermediate = if add { lhs + rhs } else { lhs - rhs };

    match divide_result_by {
        Some(div) => intermediate / div,
        None => intermediate,
    }
}

pub mod using_constants {
    /// Const in use must be at least as visible as the item using it
    pub mod consts {
        pub const SOME_DEFAULT: u32 = 4096;

        #[defamed::defamed(using_constants::consts)]
        pub struct Item(pub usize);
    }

    /// Path to item must be present for items with default constants
    #[defamed::defamed(using_constants)]
    #[derive(Debug, PartialEq)]
    pub struct WithConsts {
        pub offset: u32,

        /// A `const` keyword identifies constants
        /// The path provided to the constant must be relative to the function using it
        #[def(const consts::SOME_DEFAULT)]
        pub value: u32,
    }
}
