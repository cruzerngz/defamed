//! Function param stuff

use std::fmt::Debug;

use quote::{quote, ToTokens};
use syn::spanned::Spanned;

use crate::traits::{ToDocInfo, ToMacroPattern};

use super::{ParamAttr, PermutedItem};

/// Parsed function parameters
#[derive(Clone)]
pub struct FunctionParams {
    receiver: FnReceiver,
    pub params: Vec<FunctionParam>,
}

/// Default function parameter
#[derive(Clone)]
pub struct FunctionParam {
    /// Param name
    pat: syn::Pat,
    ty: syn::Type,
    attrs: Vec<syn::Attribute>,
    /// A const that can be used as a default value
    pub default_value: ParamAttr,
}

/// Function parameter receiver
#[derive(Clone)]
pub enum FnReceiver {
    None,
    /// Self
    Slf {
        ty: syn::Type,
        token: syn::Token![self],
        mutable: bool,
        reference: bool,
        lifetime: Option<syn::Lifetime>,
        colon_token: Option<syn::Token![:]>,
    },
}

impl Debug for FunctionParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionParam")
            .field("pat", &self.pat.to_token_stream().to_string())
            .field("ty", &self.ty.to_token_stream().to_string())
            .field("default_value", &self.default_value)
            .finish()
    }
}

impl Debug for ParamAttr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Default => write!(f, "Default"),
            Self::Expr(arg0) => write!(f, "Value({})", arg0.to_token_stream()),
            Self::Ident(arg0) => write!(f, "Ident({})", arg0.to_token_stream()),
        }
    }
}

// simple string matching
impl PartialEq for FunctionParam {
    fn eq(&self, other: &Self) -> bool {
        self.pat.to_token_stream().to_string() == other.pat.to_token_stream().to_string()
        // && self.ty == other.ty && self.default_value == other.default_value
    }
}

impl ToMacroPattern for PermutedItem<FunctionParam> {
    fn to_macro_pattern(&self) -> Option<proc_macro2::TokenStream> {
        match self {
            PermutedItem::Positional(FunctionParam { pat, .. }) => {
                let val = syn::Ident::new(&format!("{}_val", pat.to_token_stream()), pat.span());
                Some(quote! {$#val: expr})
            }
            PermutedItem::Named(FunctionParam { pat, .. }) => {
                let val = syn::Ident::new(&format!("{}_val", pat.to_token_stream()), pat.span());
                Some(quote! {#pat = $#val: expr})
            }
            PermutedItem::Default(_) => None,
        }
    }

    fn to_func_call_pattern(
        &self,
        path_prefix: Option<proc_macro2::TokenStream>,
    ) -> proc_macro2::TokenStream {
        match self {
            PermutedItem::Positional(FunctionParam { pat, .. })
            | PermutedItem::Named(FunctionParam { pat, .. }) => {
                let val = syn::Ident::new(&format!("{}_val", pat.to_token_stream()), pat.span());
                quote! {$#val}
            }
            PermutedItem::Default(FunctionParam { default_value, .. }) => {
                //
                match default_value {
                    ParamAttr::None => unimplemented!("default value must be present"),
                    ParamAttr::Default => quote! {core::default::Default::default()},
                    ParamAttr::Expr(v) => quote! {#v},
                    ParamAttr::Ident(i) => quote! {#path_prefix #i},
                }
            }
        }
    }
}

impl ToDocInfo for FunctionParam {
    fn to_doc_info(&self) -> crate::traits::DocInfo {
        crate::traits::DocInfo {
            ident: self.pat.to_token_stream().to_string(),
            ty: self.ty.to_token_stream().to_string(),
            default_value: match &self.default_value {
                ParamAttr::None => None,
                ParamAttr::Default => Some("Default::default()".to_string()),
                ParamAttr::Expr(expr) => Some(expr.to_token_stream().to_string()),
                ParamAttr::Ident(ident) => Some(ident.to_token_stream().to_string()),
            },
        }
    }
}

impl FunctionParams {
    pub fn from_punctuated(
        punctuated: syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
    ) -> Result<Self, syn::Error> {
        let mut s = Self {
            receiver: FnReceiver::None,
            params: Vec::new(),
        };
        let mut has_receiver = false;

        for punct in punctuated {
            match punct {
                syn::FnArg::Receiver(recv) => {
                    if !has_receiver {
                        has_receiver = true;
                    } else {
                        panic!("Function cannot accept multiple receivers");
                    }

                    let receiver = match (&recv.reference, &recv.mutability) {
                        (None, None) => FnReceiver::Slf {
                            ty: *recv.ty.clone(),
                            token: recv.self_token,
                            mutable: false,
                            reference: false,
                            lifetime: recv.lifetime().cloned(),
                            colon_token: recv.colon_token,
                        },
                        (None, Some(_)) => FnReceiver::Slf {
                            ty: *recv.ty.clone(),
                            token: recv.self_token,
                            mutable: true,
                            reference: false,
                            lifetime: recv.lifetime().cloned(),
                            colon_token: recv.colon_token,
                        },
                        (Some(_), None) => FnReceiver::Slf {
                            ty: *recv.ty.clone(),
                            token: recv.self_token,
                            mutable: false,
                            reference: true,
                            lifetime: recv.lifetime().cloned(),
                            colon_token: recv.colon_token,
                        },
                        (Some(_), Some(_)) => FnReceiver::Slf {
                            ty: *recv.ty.clone(),
                            token: recv.self_token,
                            mutable: true,
                            reference: true,
                            lifetime: recv.lifetime().cloned(),
                            colon_token: recv.colon_token,
                        },
                    };

                    s.receiver = receiver;
                }
                syn::FnArg::Typed(t) => {
                    let param = FunctionParam::from_pat_type(t)?;
                    s.params.push(param);
                }
            }
        }

        Ok(s)
    }

    /// Converts `Self` back to a punctuated sequence of `syn::FnArg`, with all matching inner attributes stripped.
    pub fn to_punctuated(&self) -> syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma> {
        let mut res = Vec::<syn::FnArg>::new();

        match &self.receiver {
            FnReceiver::None => (),
            FnReceiver::Slf {
                ty,
                token,
                mutable,
                reference,
                lifetime,
                colon_token,
            } => {
                res.push(syn::FnArg::Receiver(syn::Receiver {
                    attrs: Vec::new(),
                    reference: if *reference {
                        Some((Default::default(), lifetime.clone()))
                    } else {
                        None
                    },
                    mutability: if *mutable {
                        Some(Default::default())
                    } else {
                        None
                    },
                    self_token: *token,
                    colon_token: *colon_token,
                    ty: Box::new(ty.clone()),
                }));
            }
        }

        for param in &self.params {
            let pat = param.pat.to_owned();
            let ty = param.ty.to_owned();
            let s_attrs = param
                .attrs
                .iter()
                .filter(|a| !a.path().is_ident(crate::DEFAULT_HELPER_ATTR))
                .cloned()
                .collect::<Vec<_>>();

            let arg = syn::FnArg::Typed(syn::PatType {
                attrs: s_attrs,
                pat: Box::new(pat),
                colon_token: Default::default(),
                ty: Box::new(ty),
            });

            res.push(arg);
        }

        res.into_iter().collect()
    }

    /// Returns the first non-default item after the first default item, if any.
    pub fn first_invalid_param(&self) -> Option<&FunctionParam> {
        let mut iter = self
            .params
            .iter()
            .skip_while(|p| matches!(p.default_value, ParamAttr::None));

        iter.find(|f| matches!(f.default_value, ParamAttr::None))
    }
}

impl FunctionParam {
    /// Parse a type ascription pattern into `Self`.
    pub fn from_pat_type(punct: syn::PatType) -> Result<Self, syn::Error> {
        let pat = &punct.pat;
        let ty = &punct.ty;

        let default_value = ParamAttr::try_from_attrs(&punct.attrs)?;

        Ok(Self {
            pat: *pat.clone(),
            ty: *ty.clone(),
            attrs: punct.attrs,
            default_value,
        })
    }

    pub fn inner_span(&self) -> proc_macro2::Span {
        self.pat.span()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use proc_macro2::Span;
    use quote::quote;
    use syn::{punctuated::Punctuated, token::Comma, FnArg};

    #[test]
    fn test_init_params() {
        let default_ident =
            syn::Ident::new(crate::DEFAULT_HELPER_ATTR, proc_macro2::Span::call_site());

        let tokens = vec![
            quote! { a: i32 },
            quote! { b: u8 },
            quote! { #[#default_ident] c: usize },
            quote! { #[#default_ident(42)] d: i64 },
        ];

        let punct: Punctuated<FnArg, Comma> = tokens
            .into_iter()
            .map(|t| syn::parse2::<FnArg>(t).unwrap())
            .collect();

        let params = FunctionParams::from_punctuated(punct).unwrap();

        assert_eq!(params.params.len(), 4);
    }

    #[test]
    fn test_first_invalid_param() {
        let default_attr = syn::Ident::new(crate::DEFAULT_HELPER_ATTR, Span::call_site());

        let item_struct = quote! {
            fn item(
                x: i32,
                #[#default_attr] y: i32,
                z: i32
            ) {

            }
        };

        let item_fn: syn::ItemFn = syn::parse2(item_struct).unwrap();

        let fields = FunctionParams::from_punctuated(item_fn.sig.inputs).unwrap();

        let first_invalid = fields.first_invalid_param();

        println!("first invalid: {:?}", first_invalid);

        match first_invalid {
            Some(iv) => {
                assert_eq!(*iv, fields.params[2]);
            }
            None => panic!("last field must be invalid"),
        }
    }
}
