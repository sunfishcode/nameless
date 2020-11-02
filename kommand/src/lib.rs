use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    let ret = &input.sig.output;
    let name = &input.sig.ident;
    let body = &input.block;
    let asyncness = &input.sig.asyncness;
    let attrs = &input.attrs;

    if name != "main" {
        let tokens = quote_spanned! { name.span() =>
            compile_error!("only fn main can be tagged with #[kommand::main]");
        };
        return TokenStream::from(tokens);
    }

    let inputs = &input.sig.inputs;
    let result = {
        let mut args = Vec::new();
        let mut arg_names = Vec::new();
        let mut no_mut_arg_names = Vec::new();
        let mut arg_types = Vec::new();
        for input in inputs {
            let arg = match input {
                syn::FnArg::Typed(arg) => arg,
                syn::FnArg::Receiver(_) => {
                    let tokens = quote_spanned! { inputs.span() =>
                        compile_error!("fn main shouldn't take a self argument");
                    };
                    return TokenStream::from(tokens);
                }
            };

            arg_names.push(arg.pat.clone());
            arg_types.push(arg.ty.clone());

            let mut no_mut_ident = match &*arg.pat {
                syn::Pat::Ident(ident) => ident.clone(),
                _ => {
                    let tokens = quote_spanned! { inputs.span() =>
                        compile_error!("fn main should take normal named arguments");
                    };
                    return TokenStream::from(tokens);
                }
            };
            no_mut_ident.mutability = None;

            let mut no_mut_arg = arg.clone();
            no_mut_arg.pat = Box::new(syn::Pat::Ident(no_mut_ident));

            no_mut_arg_names.push(no_mut_arg.pat.clone());
            args.push(no_mut_arg);
        }
        quote! {
            #[derive(structopt::StructOpt)]
            #[structopt(name = "fixme: name", about = "fixme: about")]
            struct Opt {
                #(#args,)*
            }

            #(#attrs)*
            #[paw::main]
            #asyncness fn main(mut opt: Opt) #ret {
                #(let #arg_names = opt.#no_mut_arg_names;)*

                #body
            }

        }
    };

    result.into()
}
