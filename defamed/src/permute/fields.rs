use std::fmt;

use quote::quote;
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::traits::StripAttributes;
use crate::traits::ToMacroPattern;

use super::{ParamAttr, PermutedItem};

/// Parsed struct fields
#[derive(Clone, Debug)]
pub struct StructFields {
    ident: syn::Ident,
    pub fields: Vec<StructField>,
}

/// A parsed struct field.
/// For tuple structs, each field does not have an identitifer.
#[derive(Clone)]
pub struct StructField {
    pub vis: syn::Visibility,
    pub attrs: Vec<syn::Attribute>,
    pub ident: syn::Ident,
    pub is_tuple: bool,
    pub ty: syn::Type,
    pub default_value: ParamAttr,

    /// Overrides all other fields for [ToMacroPattern],
    /// This represents the struct update syntax without a value (`..`).
    dot_dot: bool,
}

// /// Permutation of struct fields
// #[derive(Clone)]
// pub enum PermutedField {
//     // this should be rarely used
//     Positional(StructField),
//     Named(StructField),
//     Default(StructField),
// }

impl fmt::Debug for StructField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StructField")
            // .field("vis", &self.vis)
            // .field("attrs", &self.attrs)
            .field("ident", &self.ident)
            // .field("is_tuple", &self.is_tuple)
            // .field("ty", &self.ty)
            // .field("default_value", &self.default_value)
            // .field("dot_dot", &self.dot_dot)
            .finish()
    }
}

impl PartialEq for StructField {
    fn eq(&self, other: &Self) -> bool {
        self.ident.to_token_stream().to_string() == other.ident.to_token_stream().to_string()
    }
}

// impl PartialEq for PermutedField {
//     fn eq(&self, other: &Self) -> bool {
//         let inner = match self {
//             Self::Positional(_i) => _i,
//             Self::Named(_i) => _i,
//             Self::Default(_i) => _i,
//         };

//         let othr = match other {
//             Self::Positional(_i) => _i,
//             Self::Named(_i) => _i,
//             Self::Default(_i) => _i,
//         };

//         inner == othr
//     }
// }

impl ToMacroPattern for PermutedItem<StructField> {
    fn to_macro_pattern(&self) -> Option<proc_macro2::TokenStream> {
        if self.inner().dot_dot {
            return Some(quote! {..});
        }

        match self {
            Self::Positional(StructField {
                vis,
                ident,
                is_tuple,
                ty,
                default_value,
                dot_dot,
                ..
            }) => {
                let pat = syn::Ident::new(&format!("{}_val", ident), ident.span());
                Some(quote! {$#pat: expr})
            }

            Self::Named(StructField {
                vis,
                ident,
                is_tuple,
                ty,
                default_value,
                dot_dot,
                ..
            }) => {
                // asd

                let pat = syn::Ident::new(&format!("{}_val", ident), ident.span());

                Some(quote! {#ident: $#pat: expr})
            }

            Self::Default(StructField {
                vis,
                ident,
                is_tuple,
                ty,
                default_value,
                dot_dot,
                ..
            }) => match default_value {
                ParamAttr::None => unimplemented!("default value must be present"),
                ParamAttr::Default => None,
                ParamAttr::Value(expr) => None,
            },
        }
    }

    fn to_func_call_pattern(&self) -> proc_macro2::TokenStream {
        if self.inner().dot_dot {
            return quote! {};
        }

        match self {
            PermutedItem::Positional(StructField {
                vis,
                ident,
                is_tuple,
                ty,
                default_value,
                dot_dot,
                ..
            }) => {
                // asd
                let pat = syn::Ident::new(&format!("{}_val", ident), ident.span());

                // match default_value {
                //     ParamAttr::None => quote! {#ident: $#pat},
                //     ParamAttr::Default => quote! {#ident: core::default::Default::default()},
                //     ParamAttr::Value(expr) => quote! {#ident: #expr},
                // }

                quote! {#ident: $#pat}
            }
            PermutedItem::Named(StructField {
                vis,
                ident,
                is_tuple,
                ty,
                default_value,
                dot_dot,
                ..
            }) => {
                // asd
                let pat = syn::Ident::new(&format!("{}_val", ident), ident.span());

                // match default_value {
                //     ParamAttr::None => quote! {#ident: $#pat},
                //     ParamAttr::Default => quote! {#ident: core::default::Default::default()},
                //     ParamAttr::Value(expr) => quote! {#ident: #expr},
                // }

                quote! {#ident: $#pat}
            }
            PermutedItem::Default(StructField {
                vis,
                ident,
                is_tuple,
                ty,
                default_value,
                dot_dot,
                ..
            }) => match default_value {
                ParamAttr::None => unimplemented!("default value must be present"),
                ParamAttr::Default => quote! {#ident: core::default::Default::default()},
                ParamAttr::Value(expr) => quote! {#ident: #expr},
            },
            // _ => todo!(),
        }
    }
}

impl StripAttributes for StructFields {
    type Original = syn::Fields;

    fn strip_attributes(&self) -> Self::Original {
        let x = self.fields.first().expect("at least one field expected");

        let fields = self
            .fields
            .iter()
            .map(|f| syn::Field {
                attrs: f
                    .attrs
                    .iter()
                    .cloned()
                    .filter(|a| !a.path().is_ident(crate::DEFAULT_HELPER_ATTR))
                    .collect::<Vec<_>>(),
                vis: f.vis.clone(),
                mutability: syn::FieldMutability::None,
                ident: if f.is_tuple {
                    None
                } else {
                    Some(f.ident.clone())
                },
                colon_token: Default::default(),
                ty: f.ty.clone(),
            })
            .collect();

        match x.is_tuple {
            true => syn::Fields::Unnamed(syn::FieldsUnnamed {
                paren_token: Default::default(),
                unnamed: fields,
            }),
            false => syn::Fields::Named(syn::FieldsNamed {
                brace_token: Default::default(),
                named: fields,
            }),
        }
    }
}

impl StructFields {
    /// Parse named fields
    pub fn from_named(
        ident: syn::Ident,
        fields: Punctuated<syn::Field, syn::Token![,]>,
    ) -> Result<Self, syn::Error> {
        let fields = fields
            .into_iter()
            .map(|f| StructField::from_field_type(f, None))
            .collect::<Result<_, _>>()?;

        Ok(Self { ident, fields })
    }

    /// Parse unnamed fields
    pub fn from_unnamed(
        ident: syn::Ident,
        fields: Punctuated<syn::Field, syn::Token![,]>,
    ) -> Result<Self, syn::Error> {
        let fields = fields
            .into_iter()
            .enumerate()
            .map(|(idx, field)| StructField::from_field_type(field, Some(idx)))
            .collect::<Result<_, _>>()?;

        Ok(Self { ident, fields })
    }
}

impl StructField {
    /// Parse a struct field into `Self`.
    pub fn from_field_type(
        field: syn::Field,
        tuple_elem: Option<usize>,
    ) -> Result<Self, syn::Error> {
        // look for default attr
        let mut default_value = ParamAttr::None;
        if !field.attrs.is_empty() {
            for attr in &field.attrs {
                if attr.path().is_ident(crate::DEFAULT_HELPER_ATTR) {
                    let meta = attr.meta.clone();

                    match meta {
                        syn::Meta::Path(_) => default_value = ParamAttr::Default,
                        syn::Meta::List(l) => {
                            let l_span = l.span();

                            let first_item = l.tokens.into_iter().next().ok_or(syn::Error::new(
                                l_span,
                                "expected at least 1 item in metalist",
                            ))?;

                            let e: syn::Expr = syn::parse2(first_item.to_token_stream())?;
                            default_value = ParamAttr::Value(e);
                        }
                        syn::Meta::NameValue(nv) => {
                            let e = syn::Error::new(
                                    nv.span(),
                                    format!("name-values are not supported. Use #[{}] or #[{}(CONST_EXPRESSION)] instead.",
                                        crate::DEFAULT_HELPER_ATTR,
                                        crate::DEFAULT_HELPER_ATTR
                                    ),
                                );
                            return Err(e);
                        }
                    }

                    break;
                }
            }
        };

        let res = match tuple_elem {
            Some(mut tup_id) => {
                let mut id = vec![];
                while tup_id != 0 {
                    id.push(tup_id % 26);
                    tup_id /= 10;
                }

                let tup_ident = id
                    .into_iter()
                    .map(|i| char::from_u32(i as u32 + 97).unwrap())
                    .collect::<String>();

                Self {
                    vis: field.vis,
                    attrs: field.attrs,
                    ident: syn::Ident::new(&tup_ident, field.ty.span()),
                    is_tuple: true,
                    ty: field.ty,
                    default_value,
                    dot_dot: false,
                }
            }
            None => Self {
                vis: field.vis,
                attrs: field.attrs,
                ident: field.ident.expect("named field must have an identifier"),
                is_tuple: false,
                ty: field.ty,
                default_value,
                dot_dot: false,
            },
        };

        Ok(res)
    }

    /// Constructs a `StructField` that represents `..`.
    /// All other fields are irrelevant.
    pub fn dot_dot() -> Self {
        Self {
            vis: syn::Visibility::Inherited,
            attrs: vec![],
            ident: syn::Ident::new(
                "___DOT_DOT_NO_COLLISIONS_DOT_DOT___",
                proc_macro2::Span::call_site(),
            ),
            is_tuple: false,
            ty: syn::parse_quote! {u8},
            default_value: ParamAttr::None,
            dot_dot: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    use proc_macro2::Span;
    use quote::quote;
    use syn::{punctuated::Punctuated, Ident};

    #[test]
    fn test_init_field() {
        let default_attr = syn::Ident::new(crate::DEFAULT_HELPER_ATTR, Span::call_site());

        let item_struct: syn::ItemStruct = syn::parse2(quote! {
            struct Item {
                pub x: i32,
                #[#default_attr]
                pub y: i32,
                #[#default_attr(1)]
                pub z: i32,
            }
        })
        .unwrap();

        // return;

        let fields = match item_struct.fields {
            syn::Fields::Named(fields_named) => fields_named,
            syn::Fields::Unnamed(fields_unnamed) => todo!(),
            syn::Fields::Unit => todo!(),
        };

        let fields = StructFields::from_named(item_struct.ident, fields.named).unwrap();
        // let fields = match fields {
        //     Ok(f) => f,
        //     Err(e) => {
        //         println!("error: {:?}, source: {:?}", e, e.source());
        //         panic!("parse error")
        //     }
        // };

        let inner = fields.fields;

        assert!(inner.len() == 3);
        assert!(matches!(inner[0].default_value, ParamAttr::None));
        assert!(matches!(inner[1].default_value, ParamAttr::Default));
        assert!(matches!(inner[2].default_value, ParamAttr::Value(_)));
    }

    #[test]
    fn test() {
        let x: Option<Ident> = None;
        println!("ident: {}", quote! {#x});

        // let x: (u32, u32, u32);
        // x = (1, 2, 3);
        // let y: (u32, u32, u32) = (1, 2, ..x);
    }
}
