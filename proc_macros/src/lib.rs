use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemEnum};

#[proc_macro_derive(FromArticle)]
pub fn my_macro(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as ItemEnum);

    let ident = input.ident;
    let checks = input
        .variants
        .iter()
        .map(|variant| {
            let var_ident = variant.ident.clone();
            let class_name = variant.ident.to_string();
            let class_name_up = class_name.to_ascii_uppercase();
            let class_name_lower = class_name.to_ascii_lowercase();

            quote! {
                if let Some(dist) = s.find(#class_name) {
                    if curr_dist > dist {
                        curr_dist = dist;
                        curr_class = Some(#ident::#var_ident);
                    }
                }
                if let Some(dist) = s.find(#class_name_up) {
                    if curr_dist > dist {
                        curr_dist = dist;
                        curr_class = Some(#ident::#var_ident);
                    }
                }
                if let Some(dist) = s.find(#class_name_lower) {
                    if curr_dist > dist {
                        curr_dist = dist;
                        curr_class = Some(#ident::#var_ident);
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    // Build the output, possibly using quasi-quotation
    let expanded = quote! {
        impl #ident {
            fn from_article(s: &str) -> Option<Self> {
                let mut curr_dist = usize::MAX;
                let mut curr_class = None;

                #(#checks)*

                curr_class
            }
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}

#[proc_macro_derive(AsText)]
pub fn as_text(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as ItemEnum);

    let ident = input.ident;
    let matches = input
        .variants
        .iter()
        .map(|variant| {
            let var_ident = variant.ident.clone();
            let class_name_up = variant.ident.to_string().to_ascii_uppercase();

            quote! {
                #ident::#var_ident => #class_name_up
            }
        })
        .collect::<Vec<_>>();

    // Build the output, possibly using quasi-quotation
    let expanded = quote! {
        impl #ident {
            fn as_text(&self) -> &str {
                match self {
                    #(#matches),*
                }
            }
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}
