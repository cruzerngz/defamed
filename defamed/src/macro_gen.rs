//! Function macro generators

use std::fmt::{Debug, Display};

use proc_macro2::{self as pm2, Span};
use quote::{quote, ToTokens};
use syn::{
    punctuated::Punctuated,
    token::{Comma, Semi},
    Visibility,
};

use crate::traits::{ToDocInfo, ToMacroPattern};

#[derive(Clone, Copy, Debug)]
pub enum MacroType {
    Function,
    /// Struct with named fields
    Struct,
    /// Tuple struct with unnamed fields
    StructTuple,
    // Enum,
}

/// Converts `self` to doc item disambiguation prefix
impl Display for MacroType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let item = match self {
            MacroType::Function => "fn@",
            MacroType::Struct => "struct@",
            MacroType::StructTuple => "struct@",
        };

        write!(f, "{}", item)
    }
}

/// Generate a macro with all permutations of positional, named and default parameters.
/// The macro inherits all doc comments from the original function.
///
/// This macro generates code that calls the actual function,
/// while reorderng and substituting parameters as needed.
pub fn generate_item_macro<P: ToMacroPattern + ToDocInfo + Clone + PartialEq + Debug>(
    vis: Visibility,
    // package_name: &str,
    item_path: Option<syn::Path>,
    item_ident: syn::Ident,
    params: Vec<Vec<P>>,
    output: MacroType,
) -> pm2::TokenStream {
    // first pattern contains the correct order of parameteres to call
    let first_ref = params
        .first()
        .cloned()
        .expect("at least one match pattern expected");

    let item_path_root = item_path.clone().map(|g| {
        if g.is_ident(crate::ROOT_VISIBILITY_IDENT) {
            quote! {$crate ::}
        } else {
            quote! {$crate :: #g ::}
        }
    });
    // .unwrap_or_default();

    // let package_ident = syn::Ident::new(&package_name.replace("-", "_"), Span::call_site());

    let macro_matches: Punctuated<pm2::TokenStream, Semi> = params
        .into_iter()
        .map(|p| {
            let macro_signature = create_macro_signature(&p);
            let item_signature =
                create_func_call_signature(first_ref.as_slice(), &p, item_path_root.clone());

            match output {
                MacroType::Function | MacroType::StructTuple => quote! {
                    (#macro_signature) => {
                        #item_path_root #item_ident(#item_signature)
                    }
                },
                MacroType::Struct => quote! {
                    (#macro_signature) => {
                        #item_path_root #item_ident{#item_signature}
                    }
                },
            }

            // quote! {
            //     (#macro_signature) => {
            //         #func_path_root #item_ident(#func_signature)
            //     }
            // }
        })
        .collect();

    let _macro_mod = syn::Ident::new(
        &format!("{}_macros", item_ident.to_token_stream()),
        Span::call_site(),
    );

    let macro_def_attr = match &vis {
        Visibility::Public(_) => quote! {#[macro_export]},
        Visibility::Restricted(_) | Visibility::Inherited => quote! {},
    };

    let item_dunder_ident = syn::Ident::new(
        &format!(
            "__{}{}__",
            match &item_path {
                Some(p) => format!(
                    "{}_",
                    p.segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect::<Box<[String]>>()
                        .join("_")
                ),
                None => "".to_string(),
            },
            item_ident.to_token_stream()
        ),
        Span::call_site(),
    );

    // let full_func_path = match func_path {
    //     Some(p) => quote! {crate::#p::#func_ident},
    //     None => quote! {crate::#func_ident},
    // };
    let item_prefix = output.to_string();

    let doc_type_info = first_ref
        .iter()
        .map(|p| {
            let info = p.to_doc_info().to_string();
            quote! {#[doc = concat!("- ", #info)]}
        })
        .collect::<pm2::TokenStream>();

    quote! {
        // #vis mod #macro_mod {

            #[doc(hidden)]
            #[allow(unused_macros)]
            #macro_def_attr
            macro_rules! #item_dunder_ident (
                #macro_matches
            );

            #[doc(inline)]
            #[doc = concat!("[`defamed`] wrapper for [`", #item_prefix, stringify!(#item_ident), "`]")]
            #[doc = ""]
            #doc_type_info
            #vis use #item_dunder_ident as #item_ident;

        // }
        // #vis use #macro_mod::*;
        // #vis use #func_ident!;
    }
}

// /// Struct with named fields
// pub fn generate_item_struct_struct_macro(
//     ident: syn::Ident,
//     named_fields: syn::FieldsNamed,
// ) -> pm2::TokenStream {
//     //asldkdm
//     quote! {}
// }

// /// Tuple struct
// pub fn generate_item_struct_tuple_macro(
//     ident: syn::Ident,
//     unnamed_fields: syn::FieldsUnnamed,
// ) -> pm2::TokenStream {
//     quote! {}
// }

/// Create the macro pattern signature for a given vector of parameters.
fn create_macro_signature<P: ToMacroPattern>(params: &[P]) -> pm2::TokenStream {
    let seq: Punctuated<pm2::TokenStream, Comma> =
        params.iter().filter_map(|p| p.to_macro_pattern()).collect();

    seq.to_token_stream()
}

/// Uses the reference pattern to order the parameters in the function call.
///
/// All elements in `reference` must have an equal (by [PartialEq]) in `params`.
///
/// If there are more elements in `params` than in `reference`, the extra elements are appended to the end.
fn create_func_call_signature<P>(
    reference: &[P],
    params: &[P],
    path_prefix: Option<proc_macro2::TokenStream>,
) -> pm2::TokenStream
where
    P: ToMacroPattern + PartialEq + Debug,
{
    assert!(
        reference.len() <= params.len(),
        "ref: {:?}\nparams: {:?}",
        reference,
        params
    );

    let mut seq: Punctuated<pm2::TokenStream, Comma> = reference
        .iter()
        .map(|r| {
            let p = params
                .iter()
                .find(|item| *item == r)
                .expect("parameter must exist");

            p.to_func_call_pattern(path_prefix.clone())
        })
        .collect();

    match params.len().cmp(&reference.len()) {
        std::cmp::Ordering::Less => {
            unimplemented!("reference must have at least as many elements as params")
        }
        std::cmp::Ordering::Equal => (),
        std::cmp::Ordering::Greater => {
            // todo!();
            let additional = params[reference.len()..]
                .iter()
                .map(|p| p.to_func_call_pattern(path_prefix.clone()));

            seq.extend(additional);
        }
    }

    seq.to_token_stream()
}
