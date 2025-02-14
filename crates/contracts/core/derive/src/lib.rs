use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(UserData)]
pub fn user_data_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let expanded = quote! {
        impl ::quartz_contract_core::msg::execute::attested::HasUserData for #name {
            fn user_data(&self) -> ::quartz_contract_core::state::UserData {
                use ::sha2::Digest;

                let mut hasher = ::sha2::Sha256::new();
                hasher.update(::serde_json::to_string(&self).expect("infallible serializer"));
                let digest: [u8; 32] = hasher.finalize().into();

                let mut user_data = [0u8; 64];
                user_data[0..32].copy_from_slice(&digest);
                user_data
            }
        }
    };

    TokenStream::from(expanded)
}
