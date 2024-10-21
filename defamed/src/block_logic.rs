//! Various methods for manipulating a particular block of code.

use proc_macro as pm;
use proc_macro2 as pm2;
use quote::{quote, ToTokens};

use crate::{
    macro_gen::{self, MacroType},
    permute::{
        fields::{StructField, StructFields},
        params, ParamAttr, PermutedItem,
    },
    traits::StripAttributes,
};

/// Output of a processing function
pub struct ProcOutput {
    /// Modified code to be substituted in-place
    pub modified: pm2::TokenStream,
    /// Generated code to be appended to the end of macro invocation
    pub generated: pm2::TokenStream,
}

impl From<pm::TokenStream> for ProcOutput {
    fn from(value: pm::TokenStream) -> Self {
        Self {
            modified: value.into(),
            generated: Default::default(),
        }
    }
}

impl From<pm2::TokenStream> for ProcOutput {
    fn from(value: pm2::TokenStream) -> Self {
        Self {
            modified: value,
            generated: Default::default(),
        }
    }
}

impl From<ProcOutput> for pm::TokenStream {
    fn from(value: ProcOutput) -> Self {
        let mut modified = value.modified;
        modified.extend(value.generated);

        modified.into()
    }
}

/// Process a standalone function.
/// The crate path of the funciton is passed as an optional parameter.
pub fn item_fn(input: syn::ItemFn, fn_path: Option<syn::Path>) -> ProcOutput {
    let syn::ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = input;

    // check visibility vs provided path
    match (&vis, fn_path.as_ref()) {
        (syn::Visibility::Restricted(syn::VisRestricted { path, .. }), None) => {
            if !path.is_ident("self") {
                return syn::Error::new(
                    sig.ident.span(),
                    "Attribute requires a path to the function for public functions",
                )
                .to_compile_error()
                .into();
            }
        }
        (syn::Visibility::Public(_), None) => {
            return syn::Error::new(
                sig.ident.span(),
                "Attribute requires a path to the function for public functions",
            )
            .to_compile_error()
            .into();
        }
        _ => (),
    }

    let params = match params::FunctionParams::from_punctuated(sig.inputs.clone()) {
        Ok(p) => p,
        Err(e) => return e.to_compile_error().into(),
    };

    if params
        .params
        .iter()
        .any(|p| matches!(p.default_value, ParamAttr::Ident(_)))
        && fn_path.is_none()
    {
        return syn::Error::new(
            sig.ident.span(),
            "Functions with const default values must provide a path to itself",
        )
        .to_compile_error()
        .into();
    }

    if let Some(invalid) = params.first_invalid_param() {
        return syn::Error::new(
            invalid.inner_span(),
            "Default parameters must be placed after all positional parameters",
        )
        .to_compile_error()
        .into();
    }

    let params_inner = params.params.clone();
    let (positional, default) = {
        let partition = params_inner.iter().enumerate().find_map(|(idx, f)| {
            if matches!(
                f.default_value,
                ParamAttr::Default | ParamAttr::Expr(_) | ParamAttr::Ident(_)
            ) {
                Some(idx)
            } else {
                None
            }
        });

        match partition {
            Some(p) => {
                let tup = params_inner.split_at(p);
                (tup.0.to_vec(), tup.1.to_vec())
            }
            None => (params_inner, vec![]),
        }
    };

    let permuted_new = crate::permute::permute(positional, default);
    let permuted_concat = permuted_new
        .into_iter()
        .map(|permutation| [permutation.0, permutation.1].concat())
        .collect::<Vec<_>>();

    // let permuted = params.permute_params();
    let new_args = params.to_punctuated();
    let mut new_sig = sig.clone();
    new_sig.inputs = new_args;

    // let doc_attrs = attrs
    //     .iter()
    //     .cloned()
    //     .filter(|a| a.path().is_ident("doc"))
    //     .collect::<Vec<_>>();

    let generated = macro_gen::generate_item_macro(
        vis.clone(),
        // doc_attrs,
        // package_name,
        fn_path,
        new_sig.ident.clone(),
        permuted_concat,
        macro_gen::MacroType::Function,
    );

    let mod_fn = syn::ItemFn {
        attrs,
        vis,
        sig: new_sig,
        block,
    }
    .to_token_stream();

    ProcOutput {
        modified: mod_fn,
        generated,
    }
}

/// Process a struct definition
pub fn item_struct(input: syn::ItemStruct, s_path: Option<syn::Path>) -> ProcOutput {
    match input.fields {
        syn::Fields::Named(named_fields) => item_struct_struct(
            s_path,
            input.attrs,
            input.vis,
            input.ident,
            input.generics,
            named_fields,
        ),
        syn::Fields::Unnamed(unnamed_fields) => item_struct_tuple(
            s_path,
            input.attrs,
            input.vis,
            input.ident,
            input.generics,
            unnamed_fields,
        ),
        syn::Fields::Unit => {
            let warning = proc_macro_warning::FormattedWarning::new_deprecated(
                    "IrrelevantMacro",
                    "Remove this attribute macro. Unit structs do not contain any fields and cannot have default parameters.",
                    input.ident.span(),
                );

            quote! {
                #warning
            }
            .into()
        }
    }
}

/// Process a normal struct
fn item_struct_struct(
    s_path: Option<syn::Path>,
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    ident: syn::Ident,
    generics: syn::Generics,
    fields: syn::FieldsNamed,
) -> ProcOutput {
    match (&vis, s_path.as_ref()) {
        (syn::Visibility::Restricted(syn::VisRestricted { path, .. }), p) => {
            if !fields.named.iter().all(|f| {
                matches!(
                    f.vis,
                    syn::Visibility::Public(_) | syn::Visibility::Restricted(_)
                )
            }) {
                return syn::Error::new(
                    ident.span(),
                    "Non-private structs must have non-private fields",
                )
                .to_compile_error()
                .into();
            }

            if p.is_none() && !path.is_ident("self") {
                return syn::Error::new(
                    ident.span(),
                    "Attribute requires a path to the struct for public structs",
                )
                .to_compile_error()
                .into();
            }
        }
        (syn::Visibility::Public(_), p) => {
            if !fields
                .named
                .iter()
                .all(|f| matches!(f.vis, syn::Visibility::Public(_)))
            {
                return syn::Error::new(ident.span(), "Public structs must have public fields")
                    .to_compile_error()
                    .into();
            }

            if p.is_none() {
                return syn::Error::new(
                    ident.span(),
                    "Attribute requires a path to the struct for public structs",
                )
                .to_compile_error()
                .into();
            }
        }
        (syn::Visibility::Inherited, _) => (),
    }

    let n_fields = match StructFields::from_named(ident.clone(), fields.named.clone()) {
        Ok(f) => f,
        Err(e) => return e.to_compile_error().into(),
    };

    if n_fields
        .fields
        .iter()
        .any(|f| matches!(f.default_value, ParamAttr::Ident(_)))
        && s_path.is_none()
    {
        return syn::Error::new(
            ident.span(),
            "Structs with const default fields must include a path to itself",
        )
        .to_compile_error()
        .into();
    }

    if let Some(invalid) = n_fields.first_invalid() {
        return syn::Error::new(
            invalid.ident.span(),
            "Default parameters must be placed after all positional parameters",
        )
        .to_compile_error()
        .into();
    }

    let stripped_fields = n_fields.strip_attributes();
    let fields_inner = n_fields.fields;

    let (positional, defaults) = {
        let partition = fields_inner.iter().enumerate().find_map(|(idx, f)| {
            if matches!(
                f.default_value,
                ParamAttr::Default | ParamAttr::Expr(_) | ParamAttr::Ident(_)
            ) {
                Some(idx)
            } else {
                None
            }
        });

        match partition {
            Some(p) => {
                let tup = fields_inner.split_at(p);
                (tup.0.to_vec(), tup.1.to_vec())
            }
            None => (fields_inner, vec![]),
        }

        // (0,0)
    };

    let permuted = crate::permute::permute(positional, defaults);

    let joined = permuted
        .into_iter()
        .map(|permutation| {
            let has_missing = permutation
                .1
                .iter()
                .any(|item| matches!(item, PermutedItem::Default(_)));

            match has_missing {
                true => [
                    permutation.0,
                    permutation.1,
                    vec![PermutedItem::Default(StructField::dot_dot())],
                ]
                .concat(),
                false => [permutation.0, permutation.1].concat(),
            }
        })
        .collect::<Vec<_>>();

    let generated = macro_gen::generate_item_macro(
        vis.clone(),
        s_path.clone(),
        ident.clone(),
        joined,
        MacroType::Struct,
    );

    ProcOutput {
        modified: syn::ItemStruct {
            attrs,
            vis,
            struct_token: Default::default(),
            ident,
            generics,
            fields: stripped_fields,
            semi_token: None,
        }
        .to_token_stream(),
        generated,
    }
}

/// Process a tuple struct
fn item_struct_tuple(
    s_path: Option<syn::Path>,
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    ident: syn::Ident,
    generics: syn::Generics,
    fields: syn::FieldsUnnamed,
) -> ProcOutput {
    match (&vis, s_path.as_ref()) {
        (syn::Visibility::Restricted(syn::VisRestricted { path, .. }), p) => {
            if !fields.unnamed.iter().all(|f| {
                matches!(
                    f.vis,
                    syn::Visibility::Public(_) | syn::Visibility::Restricted(_)
                )
            }) {
                return syn::Error::new(
                    ident.span(),
                    "Non-private struct tuples must have non-private items",
                )
                .to_compile_error()
                .into();
            }

            if p.is_none() && !path.is_ident("self") {
                return syn::Error::new(
                    ident.span(),
                    "Attribute requires a path to the struct tuple for public structs",
                )
                .to_compile_error()
                .into();
            }
        }
        (syn::Visibility::Public(_), p) => {
            if !fields
                .unnamed
                .iter()
                .all(|f| matches!(f.vis, syn::Visibility::Public(_)))
            {
                return syn::Error::new(
                    ident.span(),
                    "Public struct tuples must have public items",
                )
                .to_compile_error()
                .into();
            }

            if p.is_none() {
                return syn::Error::new(
                    ident.span(),
                    "Attribute requires a path to the struct for public struct tuples",
                )
                .to_compile_error()
                .into();
            }
        }
        (syn::Visibility::Inherited, _) => (),
    }

    let un_fields = match StructFields::from_unnamed(ident.clone(), fields.unnamed.clone()) {
        Ok(un) => un,
        Err(e) => return e.to_compile_error().into(),
    };

    if un_fields
        .fields
        .iter()
        .any(|f| matches!(f.default_value, ParamAttr::Ident(_)))
        && s_path.is_none()
    {
        return syn::Error::new(
            ident.span(),
            "Tuples with const default elements must include a path to itself",
        )
        .to_compile_error()
        .into();
    }

    if let Some(invalid) = un_fields.first_invalid() {
        return syn::Error::new(
            invalid.ident.span(),
            "Default parameters must be placed after all positional parameters",
        )
        .to_compile_error()
        .into();
    }

    let stripped_fields = un_fields.strip_attributes();
    let fields_inner = un_fields.fields;

    let (positional, defaults) = {
        let partition = fields_inner.iter().enumerate().find_map(|(idx, f)| {
            if matches!(
                f.default_value,
                ParamAttr::Default | ParamAttr::Expr(_) | ParamAttr::Ident(_)
            ) {
                Some(idx)
            } else {
                None
            }
        });

        match partition {
            Some(p) => {
                let tup = fields_inner.split_at(p);
                (tup.0.to_vec(), tup.1.to_vec())
            }
            None => (fields_inner, vec![]),
        }
    };

    let permuted = crate::permute::permute_tuple_struct(positional, defaults);

    let generated = macro_gen::generate_item_macro(
        vis.clone(),
        s_path.clone(),
        ident.clone(),
        permuted,
        MacroType::StructTuple,
    );

    ProcOutput {
        modified: syn::ItemStruct {
            attrs,
            vis,
            struct_token: Default::default(),
            ident,
            generics,
            fields: stripped_fields,
            semi_token: None,
        }
        .to_token_stream(),
        generated,
    }
}

#[allow(unused)]
fn impl_item_fn(input: syn::ImplItemFn) {
    // this is the only thing that is different from item_fn
    let def_ness = input.defaultness;

    let item_as_fn = syn::ItemFn {
        attrs: input.attrs,
        vis: input.vis,
        sig: input.sig,
        block: Box::new(input.block),
    };
}

/// Processes all functions inside an `impl` block
#[allow(unused)]
pub fn item_impl(input: syn::ItemImpl) -> ProcOutput {
    let inter = input.items.into_iter().map(|item| match item {
        syn::ImplItem::Const(_) => todo!(),
        syn::ImplItem::Fn(f) => {}
        syn::ImplItem::Type(_) => todo!(),
        syn::ImplItem::Macro(_) => todo!(),
        syn::ImplItem::Verbatim(_) => todo!(),
        _ => todo!(),
    });

    todo!()
}

#[allow(dead_code)]
pub fn item_mod(_input: syn::ItemMod) -> ProcOutput {
    todo!()
}

#[cfg(test)]
mod tests {
    use quote::quote;

    #[test]
    fn test_match_impl_block() {
        let tokens = quote! {
            impl SomeStruct {
                pub fn new() -> Self {
                    SomeStruct {}
                }
            }
        };

        let _: syn::ItemImpl = syn::parse2(tokens).unwrap();
    }

    #[test]
    fn test_match_mod_block() {
        let tokens = quote! {
            mod some_module {
                struct X{}
            }
        };

        let _: syn::ItemMod = syn::parse2(tokens).unwrap();
    }
}
