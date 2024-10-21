//! Permutation generation for positional and named parameters.
// #![allow(unused)]

use std::fmt::Debug;

use quote::ToTokens;
use syn::spanned::Spanned;

use crate::traits::ToDocInfo;

pub mod fields;
pub mod params;

/// Data from the `#[def]` attribute
#[derive(Clone)]
pub enum ParamAttr {
    /// No helper attribute
    None,
    /// Use default trait for initialization
    Default,
    /// Use const expr for initialization
    Expr(syn::Expr),

    /// Use a const identifier for initialization
    Ident(syn::Path),
}

/// A single permuted item
#[derive(Clone)]
pub enum PermutedItem<T: Clone> {
    /// Item is positional with no identifier
    Positional(T),
    /// Item is named with an identifier
    Named(T),
    /// Item is a default value
    Default(T),
}

impl<T: Debug + Clone> Debug for PermutedItem<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Positional(arg0) => f.debug_tuple("Positional").field(arg0).finish(),
            Self::Named(arg0) => f.debug_tuple("Named").field(arg0).finish(),
            Self::Default(arg0) => f.debug_tuple("Default").field(arg0).finish(),
        }
    }
}

impl<T: Clone + PartialEq> PartialEq for PermutedItem<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner() == other.inner()
    }
}

impl<T: Clone + ToDocInfo> ToDocInfo for PermutedItem<T> {
    fn to_doc_info(&self) -> crate::traits::DocInfo {
        self.inner().to_doc_info()
    }
}

impl ParamAttr {
    /// Searches for the correct default attribute and returns the value or a compile error.
    pub fn try_from_attrs(attrs: &[syn::Attribute]) -> Result<Self, syn::Error> {
        let mut val = Self::None;

        for a in attrs {
            if a.path().is_ident(crate::DEFAULT_HELPER_ATTR) {
                let meta = a.meta.clone();

                val = Self::try_from_attrs_inner(meta)?;
                break;
            }
        }

        Ok(val)
    }

    fn try_from_attrs_inner(meta: syn::Meta) -> Result<Self, syn::Error> {
        match meta {
            syn::Meta::Path(_) => Ok(Self::Default),
            syn::Meta::List(ml) => {
                // let l_span = ml.span();

                let list_items = ml.tokens.into_iter().collect::<Vec<_>>();

                match list_items.len() {
                    1 => {
                        let item = list_items.first().unwrap();
                        let expr = syn::parse2::<syn::Expr>(item.to_token_stream())?;

                        Ok(Self::Expr(expr))
                    }
                    _ => {
                        let (first, rest) = {
                            let mut iter = list_items.into_iter();
                            (
                                iter.next().unwrap(),
                                iter.collect::<proc_macro2::TokenStream>(),
                            )
                        };

                        if syn::parse2::<syn::Token![const]>(first.to_token_stream()).is_err() {
                            let e = syn::Error::new(
                                first.span(),
                                "expected `const` keyword before path to identifier",
                            );

                            return Err(e);
                        }

                        let path = syn::parse2::<syn::Path>(rest)?;
                        Ok(Self::Ident(path))

                        // let e = syn::Error::new(
                        //     l_span,
                        //     format!("expected 1 or 2 items in metalist, found {}", other),
                        // );
                        // Err(e)
                    }
                }
            }
            syn::Meta::NameValue(nv) => {
                let e = syn::Error::new(
                    nv.span(),
                    format!("name-values are not supported. Use #[{}], #[{}(CONST_EXPR)], #[{}(const CONST_IDENT)] instead.",
                        crate::DEFAULT_HELPER_ATTR,
                        crate::DEFAULT_HELPER_ATTR,
                        crate::DEFAULT_HELPER_ATTR,
                    ),
                );
                Err(e)
            }
        }
    }
}

impl<T: Clone> PermutedItem<T> {
    /// Get the inner value
    fn inner(&self) -> &T {
        match self {
            Self::Positional(i) => i,
            Self::Named(i) => i,
            Self::Default(i) => i,
        }
    }

    /// Split the items into used and unused default values while maintaining order.
    /// The items must only contain named and default variants.
    fn parition_named_defaults(
        items: &[PermutedItem<T>],
    ) -> (Vec<PermutedItem<T>>, Vec<PermutedItem<T>>) {
        let res: (Vec<_>, Vec<_>) = items.iter().cloned().partition(|def| match def {
            PermutedItem::Named(_) => true,
            PermutedItem::Default(_) => false,
            _ => panic!("unexpected variant"),
        });

        res
    }
}

/// Generate all permutations of positional items and default items.
///
/// Returns a matrix of tuples of positional and default permutations.
///
/// The first permutation in the permutation matrix is guraranteed to contain
/// only [PermutedItem::Named] elements in the original order (`required`, `default` concatenated).
#[allow(clippy::type_complexity)]
pub fn permute<T: Clone + Debug>(
    required: Vec<T>,
    default: Vec<T>,
) -> Vec<(Vec<PermutedItem<T>>, Vec<PermutedItem<T>>)> {
    let named_permute = (0..=required.len())
        .flat_map(|idx| {
            let (positional, named) = required.split_at(idx);

            let positional = positional
                .iter()
                .map(|p| PermutedItem::Positional(p.to_owned()))
                .collect::<Vec<_>>();
            let permute_slice = permute_named(named);

            permute_slice
                .iter()
                .map(|named_seq| [positional.as_slice(), named_seq.as_slice()].concat())
                .collect::<Vec<_>>()
                .into_iter()
        })
        .collect::<Vec<_>>();

    debug_assert!(if let Some(l) = named_permute.last() {
        l.iter()
            .all(|item| matches!(item, PermutedItem::Positional(_)))
    } else {
        true
    });

    let default_permute = permute_named_default(&default);
    let default_positional_permute = permute_pos_default(&default);

    // last element in named permutation matrix contains all positional parameters
    let all_positional = match named_permute.last() {
        Some(base) => {
            // sanity check
            assert!(
                base.iter()
                    .all(|item| matches!(item, PermutedItem::Positional(_))),
                "all permuted items in the last permuted sequence must be positional."
            );

            default_positional_permute
                .into_iter()
                .map(|seq| (base.to_vec(), seq.to_vec()))
                .collect::<Vec<_>>()
        }
        None => default_positional_permute
            .into_iter()
            .map(|seq| (vec![], seq))
            .collect(),
    };

    // constructing intermediate permutations w/ named and default parameters
    let named_pos = match (named_permute.len(), default_permute.len()) {
        // we do not append completely empty sequences
        (0, 0) => return all_positional,
        (0, _) => default_permute
            .into_iter()
            .map(|seq| (vec![], seq))
            .collect::<Vec<_>>(),
        (_, 0) => named_permute
            .into_iter()
            .map(|seq| (seq, vec![]))
            .collect::<Vec<_>>(),
        _ => named_permute
            .into_iter()
            .flat_map(|seq| {
                default_permute
                    .iter()
                    .map(move |def_seq| (seq.to_vec(), def_seq.to_vec()))
            })
            .collect::<Vec<_>>(),
    };

    // append default positional special cases to the end
    [named_pos, all_positional].concat()
}

/// Special permutation case for tuple structs.
///
/// Tuple structs elements are positional only.
/// Default parameters are permuted as positionals.
pub fn permute_tuple_struct<T: Clone>(
    required: Vec<T>,
    default: Vec<T>,
) -> Vec<Vec<PermutedItem<T>>> {
    let positionals = required
        .into_iter()
        .map(|f| PermutedItem::Positional(f))
        .collect::<Vec<_>>();

    let res = (0..default.len() + 1)
        .map(|default_idx| {
            let (def_pos, def_unused) = default.split_at(default_idx);
            let def_pos_perm = def_pos
                .iter()
                .cloned()
                .map(|f| PermutedItem::Positional(f))
                .collect::<Vec<_>>();
            let def_unused_perm = def_unused
                .iter()
                .cloned()
                .map(|f| PermutedItem::Default(f))
                .collect::<Vec<_>>();

            [positionals.clone(), def_pos_perm, def_unused_perm].concat()
        })
        .collect::<Vec<_>>();

    res
}

/// Perform permutations of all items in a slice.
/// All items will be wrapped in [PermutedItem::Named].
fn permute_named<T: Clone>(named: &[T]) -> Vec<Vec<PermutedItem<T>>> {
    let permutations = permute::permutations_of(named);

    permutations
        .into_iter()
        .map(|single_perm| {
            single_perm
                .into_iter()
                .map(|item| PermutedItem::Named(item.clone()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}

/// Perform permutations for default parameters. All permuted values are named.
/// For permutations of positional defaults see [permute_pos_default].
///
/// The first permutation in the matrix is in the original order of the input.
///
/// Each item in the slice must have a default value.
/// This function will not check for this.
///
/// Additionally, default params can be used(named) or unused(default). These are also permuted as well.
fn permute_named_default<T: Clone + Debug>(defaults: &[T]) -> Vec<Vec<PermutedItem<T>>> {
    let base_permute = (0..(1 << defaults.len()))
        .rev()
        .map(|num| {
            let seq = defaults
                .iter()
                .enumerate()
                .map(|(pos, item)| {
                    // if bit set, it is used
                    if (num >> pos) & 1 != 0 {
                        PermutedItem::Named(item.to_owned())
                    } else {
                        PermutedItem::Default(item.to_owned())
                    }
                })
                .collect::<Vec<_>>();

            seq
        })
        .collect::<Vec<_>>();

    let res = base_permute
        .into_iter()
        .flat_map(|seq| {
            let (used, unused) = PermutedItem::<T>::parition_named_defaults(&seq);

            let mut used_permute = permute::permute(used);

            if !unused.is_empty() {
                for item in &mut used_permute {
                    item.extend_from_slice(&unused);
                }
            }

            used_permute.into_iter()
        })
        .collect::<Vec<_>>();

    res.into_iter().filter(|item| !item.is_empty()).collect()
}

/// Perform permutations for positional default permutations.
///
/// This is for the special case where all preceding non-default parameters
/// are used as positional parameters.
fn permute_pos_default<T: Clone + Debug>(defaults: &[T]) -> Vec<Vec<PermutedItem<T>>> {
    let res = (1..=defaults.len())
        .flat_map(|idx| {
            let (positional, named) = defaults.split_at(idx);
            let pos_params = positional
                .iter()
                .map(|p| PermutedItem::Positional(p.to_owned()))
                .collect::<Vec<_>>();

            let inter = match named.len() {
                0 => vec![pos_params],
                _ => {
                    let named_permute = permute_named_default(named);

                    named_permute
                        .into_iter()
                        .map(move |named_seq| [pos_params.clone(), named_seq].concat())
                        .collect::<Vec<_>>()
                }
            };

            inter.into_iter()
        })
        .collect::<Vec<_>>();

    res
}

#[cfg(test)]
mod tests {

    use super::*;

    /// Idx to valid rust ident
    fn idx_to_str(mut idx: usize) -> String {
        let mut id = vec![];
        if idx == 0 {
            id.push(0);
        }

        while idx != 0 {
            id.push(idx % 26);
            idx /= 26;
        }

        id.into_iter()
            .map(|i| char::from_u32(i as u32 + 97).unwrap())
            .collect::<String>()
    }

    #[test]
    fn test_permute_tuple_struct() {
        fn assert_positional_default_split_at<T: Clone>(
            permutation: &[PermutedItem<T>],
            split: usize,
        ) {
            let (positional, default) = permutation.split_at(split);
            assert!(positional
                .iter()
                .all(|f| matches!(f, PermutedItem::Positional(_))));
            assert!(default
                .iter()
                .all(|f| matches!(f, PermutedItem::Default(_))));
        }

        let positional = vec!["a", "b"];
        let defaults = vec!["c", "d"];

        let permutations = permute_tuple_struct(positional, defaults);

        assert_eq!(permutations.len(), 3);
        assert_positional_default_split_at(&permutations[0], 2);
        assert_positional_default_split_at(&permutations[1], 3);
        assert_positional_default_split_at(&permutations[2], 4);
    }

    /// Test inner named permute function
    #[test]
    fn test_permute_inner_named() {
        let items = vec!["a", "b", "c", "d"];

        let permutations = permute_named(&items);
        // println!("{:?}", permutations);
        assert_eq!(permutations.len(), 24);

        let _ = permutations
            .into_iter()
            .flatten()
            .map(|item| {
                assert!(matches!(item, PermutedItem::Named(_)));
            })
            .collect::<Vec<_>>();
    }

    /// Test inner default permute function
    #[test]
    fn test_permute_inner_named_defaults() {
        let mut items = vec!["a", "b"];

        let permutations = permute_named_default(&items);

        // 0 0
        // 0 1
        // 1 0
        // 1 1
        // 1 1 again because used defaults have to be permuted
        assert_eq!(permutations.len(), 5);

        let first = permutations.first().unwrap();
        assert!(first
            .iter()
            .all(|item| matches!(item, PermutedItem::Named(_))));
        assert_eq!(
            *first,
            vec![PermutedItem::Named("a"), PermutedItem::Named("b"),]
        );

        items.clear();
        let permutations = permute_pos_default(&items);
        assert!(permutations.is_empty());
    }

    #[test]
    fn test_permute_inner_named_defaults_9() {
        let items = (0..9).map(idx_to_str).collect::<Vec<_>>();

        for i in 1..=9 {
            let inputs = &items[..i];
            let permutations = permute_named_default(inputs);
            let first = permutations.first().unwrap();

            println!("9 defaults: {} branches", permutations.len());
            println!("first item: {:?}", first);

            let expected = inputs
                .iter()
                .map(|item| PermutedItem::Named(item.to_owned()))
                .collect::<Vec<_>>();

            assert_eq!(
                expected, *first,
                "first permutation must have the same order as the input"
            );
        }
    }

    /// Test inner positional defaults permute function
    #[test]
    fn test_permute_inner_positional_defaults() {
        let items = vec!["a", "b", "c"];

        let permutations = permute_pos_default(&items);

        // 0 0
        // 0 1
        // 1 0
        // 1 1
        assert_eq!(permutations.len(), 8);
        let first = permutations.first().unwrap();

        assert!(matches!(first[0], PermutedItem::Positional(_)));
        assert!(matches!(first[1], PermutedItem::Named(_)));
        assert!(matches!(first[2], PermutedItem::Named(_)));

        assert_eq!(
            *first,
            vec![
                PermutedItem::Positional("a"),
                PermutedItem::Named("b"),
                PermutedItem::Named("c")
            ]
        );
    }

    /// Test full permutation
    #[test]
    fn test_permute_positional_named() {
        let items = vec!["a", "b", "c", "d"];

        let permutations = permute(items, vec![]);

        let first_perm = permutations.first().unwrap();

        assert!(
            first_perm
                .0
                .iter()
                .all(|item| matches!(item, PermutedItem::Named(_))),
            "first permutation must contain all named parameters"
        );
        assert!(
            first_perm
                .1
                .iter()
                .all(|item| matches!(item, PermutedItem::Named(_))),
            "first permutation must contain all named parameters"
        );
        let full_perm = [first_perm.0.clone(), first_perm.1.clone()].concat();

        assert_eq!(
            full_perm,
            vec![
                PermutedItem::Named("a"),
                PermutedItem::Named("b"),
                PermutedItem::Named("c"),
                PermutedItem::Named("d"),
            ],
            "first permutation must maintain the initial order"
        );
        assert_eq!(permutations.len(), 34);
    }

    /// Test positional and default parameters
    #[test]
    fn test_permute_positional_default() {
        let items = vec!["a", "b", "c", "d"];
        let defaults = vec!["e", "f"];

        let permutations = permute(items, defaults);

        let first_perm = permutations.first().unwrap();

        assert!(
            first_perm
                .0
                .iter()
                .all(|item| matches!(item, PermutedItem::Named(_))),
            "first permutation must contain all named parameters"
        );
        assert!(
            first_perm
                .1
                .iter()
                .all(|item| matches!(item, PermutedItem::Named(_))),
            "first permutation must contain all named parameters"
        );

        let full_perm = [first_perm.0.clone(), first_perm.1.clone()].concat();
        assert_eq!(
            full_perm,
            vec![
                PermutedItem::Named("a"),
                PermutedItem::Named("b"),
                PermutedItem::Named("c"),
                PermutedItem::Named("d"),
                PermutedItem::Named("e"),
                PermutedItem::Named("f"),
            ],
            "first permutation must maintain the initial order"
        );

        // 5 permutations for default parameters
        // and 3 additional permutations for positional default parameters (not permuted)
        assert_eq!(permutations.len(), 34 * 5 + 3);
    }

    /// Test 1-10 positional parameters
    #[test]
    fn test_permute_9_positional() {
        for i in 1..=9 {
            let items = (0..i).map(idx_to_str).collect::<Vec<_>>();

            let permutations = permute(items.clone(), vec![]);

            println!("{} positionals: {} branches", i, permutations.len());

            let first_perm = permutations.first().unwrap();
            assert_eq!(
                first_perm.0,
                items.clone()
                    .into_iter()
                    .map(|item| PermutedItem::Named(item.to_owned()))
                    .collect::<Vec<_>>(),
                "permutation for {} positional params: first permutation must have the same order as the input",
                i
            );
        }
    }

    /// Test 1-10 default parameters
    #[test]
    fn test_permute_9_default() {
        for i in 1..=9 {
            let items = (0..i).map(idx_to_str).collect::<Vec<_>>();
            let permutations = permute(vec![], items.clone());

            println!("{} defaults: {} branches", i, permutations.len());

            let first_perm = permutations.first().unwrap();
            assert_eq!(
                    first_perm.1,
                    items.clone()
                        .into_iter()
                        .map(|item| PermutedItem::Default(item.to_owned()))
                        .collect::<Vec<_>>(),
                    "permutation for {} default params: first permutation must have the same order as the input",
                    i
                );
        }
    }

    #[test]
    fn test_permute_9_default_positional() {
        const NUM: usize = 9;

        let items = (0..NUM).map(idx_to_str).collect::<Vec<_>>();

        for i in 0..=NUM {
            let (pos, def) = items.split_at(i);

            let permutations = permute(pos.to_vec(), def.to_vec());

            println!(
                "pos: {}, def: {}\tbranches: {}",
                pos.len(),
                def.len(),
                permutations.len()
            );

            let first_perm = permutations.first().unwrap();

            assert_eq!(
                first_perm.0,
                pos.iter()
                    .map(|item| PermutedItem::Named(item.to_owned()))
                    .collect::<Vec<_>>(),
                "pos: {}, def: {}: first permutation must have the same order as the input",
                pos.len(),
                def.len()
            );
        }
    }

    #[test]
    fn test_ident() {
        let token = quote::quote! { path::to::CONST };

        let expr = syn::parse2::<syn::Path>(token).unwrap();

        println!("{}", expr.to_token_stream());
    }
}
