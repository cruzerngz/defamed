# defamed
<div align="center">

#### Default, named and positional parameters.

[**Functions**](#functions) |
[**Structs**](#structs)

[![crate](https://img.shields.io/crates/v/defamed.svg)](https://crates.io/crates/defamed)
[![docs](https://docs.rs/defamed/badge.svg)](https://docs.rs/defamed)
[![build status](https://github.com/cruzerngz/defamed/actions/workflows/tests.yml/badge.svg)](https://github.com/cruzerngz/defamed/actions/workflows/tests.yml)

</div>

## Quick start
Tag supported items with `#[defamed::defamed]`, default parameters with `#[def]`
and use the generated macro with any combination of positional and named parameters.

### Functions
```rust
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
        None => intermediate
    }
}

assert_eq!(10, complex_function!(5, 5));
assert_eq!(10, complex_function!(rhs = 5, lhs = 15, add = false));
assert_eq!(5, complex_function!(5, 5, divide_result_by = Some(2)));
```

### Structs
Struct macros can be used in-place of the [builder pattern](https://crates.io/crates/derive_builder).

```rust
/// A struct that does not fully implement core::default::Default
#[defamed::defamed]
#[derive(Debug, PartialEq)]
struct PartialDefault<'a> {
    pub inner: &'a [u8],

    #[def]
    pub idx: usize,

    #[def]
    pub len: usize,
}

// use struct update syntax `..` to fill in all remaining fields
let pd = PartialDefault! {inner: &[1, 2, 3], idx: 1, ..};
let reference = PartialDefault { inner: &[1, 2, 3], idx: 1, len: 0 };

assert_eq!(reference, pd);

/// Tuple structs work very similarly to functions - but all parameters are positional
#[derive(Debug, PartialEq)]
#[defamed::defamed]
struct TupleStruct(i32, #[def] i32, #[def] bool);

let ts_a = TupleStruct!(5);
let ts_b = TupleStruct!(5, 0);
let reference = TupleStruct(5, 0, false);

assert_eq!(reference, ts_a);
assert_eq!(reference, ts_b);
```

## Features
- Named and positional parameters in any order à la [Python](https://docs.python.org/3/tutorial/controlflow.html#more-on-defining-functions)
- Define defaults as literals, constants or types implementing `Default`
- Generated macros live in the same path as the associated item
- Export macros for use in other crates
- With the heavy lifting done at compile time

## Similar crates
#### [named](https://docs.rs/named)
- locally scoped macros only
#### [default_args](https://docs.rs/default_args)
- accepts python-like syntax in function signature
- may cause code clutter due to using function macros
#### [duang](https://docs.rs/duang)
- similar to `default_args`
#### [optargs](https://docs.rs/optargs)
- infers optional parameters that use the Option type
- struct builder macro
#### [default_kwargs](https://docs.rs/default_kwargs)
- keyword argument passing only
#### [nade](https://docs.rs/nade)
- macro requires explicit import to call underlying function

## Default parameters
Default parameters can be defined in 3 ways:
- as a type implementing `Default`
- as a literal expression
- as a constant

> [!NOTE]
> This syntax applies to all supported items.

### Default trait and literal expressions
```rust
#[defamed::defamed]
fn fn_with_defaults(
    #[def] input: usize,
    #[def(3)] literal: usize,
) -> usize {
    input << literal
}

assert_eq!(0, fn_with_defaults!());
assert_eq!(8, fn_with_defaults!(1));
assert_eq!(4, fn_with_defaults!(1, 2));
assert_eq!(32, fn_with_defaults!(literal = 5, input = 1));
```

### Constants
```rust ,ignore
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
        /// Paths are relative to the item using the constant
        #[def(const consts::SOME_DEFAULT)]
        pub value: u32,
    }
}

let reference = using_constants::WithConsts {
    offset: 404,
    value: 4096,
};
assert_eq!(reference, using_constants::WithConsts! { offset: 404, .. });
```

## Parameter passing
The macro accepts parameters in any permutation as long as the following conditions are met:
- positional parameters order follows the original function signature
- all positional parameters are passed first
- named parameters come after all positional parameters
- named parameters can be included in any order
- default parameters are passed last
- default parameters can be excluded

<details>

<summary>Example</summary>

```rust
/// Add/sub 2 numbers, then take the absolute value, if applicable
#[defamed::defamed]
fn pos_and_def(
    lhs: i32,
    rhs: i32,
    #[def(true)]
    add: bool,
    #[def]
    abs_val: bool
) -> i32 {
    let inter = if add {lhs + rhs} else {lhs - rhs};
    if abs_val {inter.abs()} else {inter}
}

// original fn
assert_eq!(20, pos_and_def(5, 15, true, false));

// all positional
assert_eq!(20, pos_and_def!(5, 15, true, false));
// all named
assert_eq!(20, pos_and_def!(lhs=5, rhs=15, add=true, abs_val=false));
// all named, in any order, defaults last
assert_eq!(20, pos_and_def!(rhs=15, lhs=5, abs_val=false, add=true));
// defaults excluded
assert_eq!(20, pos_and_def!(5, 15));
// defaults excluded, positional in any order
assert_eq!(20, pos_and_def!(rhs=15, lhs=5));
// some positional, some named
assert_eq!(20, pos_and_def!(5, rhs=15));

// overriding first default parameter as positional
assert_eq!(20, pos_and_def!(25, 5, false));
// overriding second default parameter as named
assert_eq!(20, pos_and_def!(5, -25, abs_val=true));
```

</details>

## Macro scope
Macros generated by `defamed` can be exported and used by other crates if the path to the underlying function is public.

### Private
For functions that are used in the same module as they are defined, the macro resolves the call directly.
```rust ,ignore
#[defamed::defamed]
fn local_scope() {}
// macro resolves to:
local_scope!() => local_scope()
```

When private functions are used inside child modules, the module path relative to the crate root needs to be provided.
```rust ,ignore
#[defamed::defamed(crate)]
fn top_level_local_scope() {}

mod inner_consumer {
    fn inner() {
        super::top_level_local_scope!()
    }
}
```

### Public or Restricted
Functions with non-private visibility are called with their corresponding fully qualified path relative to the crate root.
The macro will require the module path to the function relative to the crate root.

For functions defined in the crate root, use `crate` as a path instead.
These macros can be used inside or imported by other crates as the macro [substitutes metavariables](https://doc.rust-lang.org/reference/macros-by-example.html#hygiene).

```rust ,ignore
// public vis in root scope
#[defamed::defamed(crate)]
pub fn root_scope() {}

pub mod inner {
    // restricted vis in module `inner`
    #[defamed::defamed(inner)]
    pub(crate) fn crate_scope() {}
}

// macros resolve to:
crate_scope!() => $crate::inner::crate_scope()
root_scope!() => $crate::root_scope()
```

### Struct field visibility
Struct fields must be at least as visible as the struct itself.
Public structs may be constructed by external crates, so the macro will require all fields to be public.

Valid examples:
```rust
#[defamed::defamed]
struct Private {
    field: i32
}

// restricted visibility or higher
#[defamed::defamed(crate)]
pub(crate) struct Restricted {
    #[def]
    pub field: i32
}

#[defamed::defamed(crate)]
pub struct Public {
    #[def]
    pub field: i32
}

#[defamed::defamed(crate)]
pub struct PublicTuple(pub i32, #[def] pub i32);
```

Invalid examples:
```rust ,compile_fail
#[defamed::defamed(crate)]
pub struct Public {
    pub field: i32
}

#[defamed::defamed(crate)]
pub struct InvalidOrder {
    /// default fields must be defined last - compile error
    #[def]
    pub field_a: i32,
    pub field_b: u32,
}

// all fields must be public
#[defamed::defamed(crate)]
pub struct PublicTuple(pub i32, #[def] i32);

// will not compile - unit structs do not have any fields
#[defamed::defamed]
struct UnitStruct;
```

## Macro generation size
> [!CAUTION]
> The size of the macro generated (number of match arms) is exponentially related to $max(positional, default)$.
> This is because the macro contains all permutations of positional and default parameters.

It is recommended that items do not exceed 9 positional and/or 9 default parameters.
Exceeding this number **will** cause the build times to increase significantly.

## Benefits
- Better ergonomics
- More clarity during code reviews
- Seamless addition of default parameters to existing items without breaking compatibility

## Limitations
- applicable for standalone functions defined outside of an `impl` block
- requires specifying fully qualified module path to item
- renaming parameters requires updating all macro invocations

<!-- ## Notes 4 me
- Determine macro invocation semantics
    - no DSL (function macros only)
    - attr macro w/ pseudo helper-attrs
- Determine param permutations a-la Python
- Exporting macro in module (! @ crate root) based on visibility:
    - main issue: https://github.com/rust-lang/rust/issues/59368
    - fix: https://github.com/rust-lang/rust/pull/108241
    - rust-analyzer hinting has issues
- Problem when invoking macro from extern module/crate
    - similar crates do not export macro with function (named, etc..)
    - inner function requires fully qualified path
    - attempt 1: module_path!() macro
        - macro needs to expand after insertion in attributed code
        - parse &str from compiler builtin macro
        - macros evaluate lazily -> outer macro receives ItemMacro tokens
        - possible, but requires nightly
    - other attempts:
        - caller_modpath: https://docs.rs/caller_modpath/latest/caller_modpath/
            - also requires nightly
        - eager: https://docs.rs/eager/latest/eager/macro.eager.html
            - does not expand builtin macro
    - crate name eval can be done at compile time using proc-macros
        - evaluate "CARGO_PKG_NAME" env var inside macro

- Current (temp) solution: define crate path path as a parameter in attribute

- New (iffy) solution: multi stage macros
    - this solution requires that this library is also included by the user in their crate (double import)
    - first proc-macro generates actual function macro with all permutations and exports function macro under module scope
    - when called, function macro resolves to another proc-macro to eval crate root path (crate:: or otherwise). this proc-macro is provided by this crate, hence the need to double import
    - final function substituted in code

- New (less iffy solution): more macro permutations!
    - every macro permutation now has 2 variants: a crate-wide invocation and a public invocation.
    - any macro not called in the same scope as it was defined will need the fully qualified path of it's invoked inner function
    - a `crate:` prefix indicates that the macro substitutes code for invocation inside it's own crate
    - no prefix indicates that code should be substituted for users of that crate

- New (fr fr) solution: macro metavariables
    - $crate metavariable forces the compiler to perform lookup in the original crate
    - attribute macro still requires the module path to the item


-->