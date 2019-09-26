#![recursion_limit = "128"]
#![feature(proc_macro_diagnostic)]

extern crate proc_macro;
use self::proc_macro::TokenStream;

use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Expr, Ident, Token, Type, Visibility};

/// Parses the following syntax, which aligns with the input of the real
/// `lazy_static` crate.
///
///     lazy_static! {
///         $VISIBILITY static ref $NAME: $TYPE = $EXPR;
///     }
///
/// For example:
///
///     lazy_static! {
///         static ref USERNAME: Regex = Regex::new("^[a-z0-9_-]{3,16}$").unwrap();
///     }
struct LazyStatic {
    name: Ident,
    ty: Type,
    init: Expr,
}

impl Parse for Vec<LazyStatic> {
    fn parse(input: ParseStream) -> Result<Self> {
        let visibility: Visibility = input.parse()?;
        input.parse::<Token![static]>()?;
        input.parse::<Token![ref]>()?;
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty: Type = input.parse()?;
        input.parse::<Token![=]>()?;
        let init: Expr = input.parse()?;
        input.parse::<Token![;]>()?;
        Ok(LazyStatic {
            name,
            ty,
            init,
        })
    }
}

#[proc_macro]
pub fn lazy_static(input: TokenStream) -> TokenStream {
    let LazyStatic {
        name,
        ty,
        init,
    } = parse_macro_input!(input as LazyStatic);

    let init_ptr = quote_spanned! {init.span()=>
        Box::into_raw(Box::new(#init))
    };

    let expanded = quote! {
        #visibility struct #name;

        impl std::ops::Deref for #name {
            type Target = #ty;

            fn deref(&self) -> &#ty {
                #assert_sync
                #assert_sized

                static ONCE: std::sync::Once = std::sync::Once::new();
                static mut VALUE: *mut #ty = 0 as *mut #ty;

                unsafe {
                    ONCE.call_once(|| VALUE = #init_ptr);
                    &*VALUE
                }
            }
        }
    };

    TokenStream::from(expanded)
}
