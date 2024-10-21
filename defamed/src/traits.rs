use proc_macro2 as pm2;

/// Turn `Self` into fragments of rust code.
pub trait ToMacroPattern {
    /// Create a macro match pattern.
    ///
    /// ```ignore
    /// macro_rules! some_macro (
    ///     ($pos_a_val: expr, $pos_b_val: expr) => {
    ///     //^^^^^^^^^^^^^^^ this is one pattern
    ///     // a macro match pattern consists of one or more comma-separated patterns.
    ///     // params that contain a default value and are not used should return None.
    ///
    ///     }
    /// )
    /// ```
    fn to_macro_pattern(&self) -> Option<pm2::TokenStream>;

    /// Create a function call pattern.
    /// Any additional path is prefixed to the symbol.
    ///
    /// ```ignore
    /// macro_rules! some_macro (
    ///     ($pos_a_val: expr, $pos_b_val: expr) => {
    ///         function_call($pos_a_val, $pos_b_val)
    ///     //                ^^^^^^^^^^ this is one pattern
    ///     // a function call pattern consists of one or more comma-separated patterns
    ///     }
    /// )
    /// ```
    fn to_func_call_pattern(&self, path_prefix: Option<pm2::TokenStream>) -> pm2::TokenStream;
}

/// Strip matching attributes from a type.
/// For function parameters, this is the `#[def]` attribute.
pub trait StripAttributes {
    /// The original type
    type Original;

    fn strip_attributes(&self) -> Self::Original;
}

/// Documentation info for an item
#[derive(Clone, Debug)]
pub struct DocInfo {
    pub ident: String,
    pub ty: String,
    pub default_value: Option<String>,
}

impl std::fmt::Display for DocInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.default_value {
            Some(val) => write!(f, "`{}`: `{}` = `{}` ", self.ident, self.ty, val),
            None => write!(f, "`{}`: `{}` ", self.ident, self.ty),
        }
    }
}

/// Document `Self`.
pub trait ToDocInfo {
    fn to_doc_info(&self) -> DocInfo;
}

// /// Generate all permutations of a set of items.
// pub trait GeneratePermutations {
//     type Item;

//     /// Generate all permutations of the items.
//     fn generate_permutations(&self) -> Vec<Vec<Self::Item>>;
// }
