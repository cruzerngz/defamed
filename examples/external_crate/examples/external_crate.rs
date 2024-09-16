//! This example shows how to import `defamed` macros from a library crate.
extern crate defamed_test_lib;

use defamed_test_lib::inner;

fn main() {
    // macros live in the same path as their original functions
    let _ = defamed_test_lib::inner::nested_inner_function(0b111, 0b100, 2);

    let a = inner::nested_inner_function!(1, 1);
    let b = inner::nested_inner_function!(0b111, 0b100, 2);

    println!("1 & 1 shifted 0 times has LSB: {}", a);
    println!("7 & 4 shifted 2 times has MSB: {}", b);

    let r_1 = defamed_test_lib::some_root_function("base", None);
    let r_2 = defamed_test_lib::some_root_function!("base");
    assert_eq!(r_1, r_2);
}
