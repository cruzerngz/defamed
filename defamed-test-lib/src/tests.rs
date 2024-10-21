#![cfg(test)]
use super::*;

#[test]
fn test_complex_function() {
    let _ = complex_function!(1, 2);

    assert_eq!(complex_function!(10, 5), 15);
    assert_eq!(complex_function!(10, 5, add = false), 5);
    assert_eq!(complex_function!(10, 20, divide_result_by = Some(2)), 15);

    // all arguments can be named
    assert_eq!(
        complex_function!(lhs = 20, rhs = 10, add = false, divide_result_by = Some(2)),
        5
    );
    // positional arguments can be named in any order, but must be provided before default arguments
    assert_eq!(
        complex_function!(rhs = 10, lhs = 20, divide_result_by = Some(2), add = false),
        5
    );
    assert_eq!(complex_function!(20, 10, false, Some(2)), 5);
}

#[test]
fn test_default_struct() {
    let a = DefaultStruct! {
        index: 0,
        ..
    };

    let b = DefaultStruct! {
        index: 1,
        inner: &[1, 2, 3],
        ..
    };

    let c = DefaultStruct! {
        index: 1,
        offset: 1,
        inner: &[1, 2, 3]
    };

    assert_eq!(a.value_at(), None);
    assert_eq!(b.value_at(), Some(2));
    assert_eq!(c.value_at(), Some(3));
}

#[test]
fn test_default_tuple_struct() {
    let a = DefaultTupleStruct!(1);
    let b = DefaultTupleStruct!(1, 2);
    let c = DefaultTupleStruct!(2, 4, 'f');

    assert_eq!(a, DefaultTupleStruct(1, 0, 'a'));
    assert_eq!(b, DefaultTupleStruct(1, 2, 'a'));
    assert_eq!(c, DefaultTupleStruct(2, 4, 'f'));
}

/// This test runs code that is identically defined in a doctest in README.md
///
/// Test const defaults from doctest as doctests do not resolve `crate` paths
/// in the same way as the main crate.
#[test]
fn test_doctest_clone() {
    let reference = using_constants::WithConsts {
        offset: 404,
        value: 4096,
    };
    assert_eq!(reference, using_constants::WithConsts! { offset: 404, .. });
}

#[test]
fn test_all_default() {
    // let a = all_default!();
    // let b = all_default!(1);
    // let c = all_default!(1, 2);
    // let d = all_default!(1, 2, 3);
    // // let e = all_default!(1, 2, 3, 4);
    // // let f = all_default!(1, 2, 3, 4, 5);

    // let identical = [a, b, c, d];
    // assert!(identical.iter().all(|i| *i == 15));

    // all_default!(e = 1, a = 2, c = 3, d = 4, b = 5);
    // all_default!(c = 1, d = 3, b = 2);
    // all_default!(a = 1, b = 2, c = 3, d = 4, e = 5);
}
