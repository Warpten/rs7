extern crate proc_macro;

use syn::DeriveInput;

pub fn bytecode_insn_impl(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    use quote::quote;
    use syn::parse2;

    let ast: DeriveInput = parse2(input).expect("Failed to parse input");

    let name = &ast.ident;

    // Extract enum variants
    let variants = match ast.data {
        syn::Data::Enum(ref data_enum) => &data_enum.variants,
        _ => panic!("EnumIter can only be used on enums"),
    };

    let arms = (0u32..).zip(variants.iter()).map(|(i, v)| {
        let ident = &v.ident;

        let fields = match &v.fields {
            syn::Fields::Named(named) => &named.named,
            _ => panic!("Bytecode instruction only supports named fields"),
        };

        let decoded_fields = fields.iter().map(|f| {
            let field_ident = f.ident.as_ref().unwrap();
            let name_str = field_ident.to_string();

            let expr = match name_str.as_str() {
                "a" => quote! { ((insn >> 8) & 0xFF) as u8 },
                "b" => quote! { ((insn >> 16) & 0xFF) as u8 },
                "c" => quote! { ((insn >> 24) & 0xFF) as u8 },
                "d" => quote! { ((insn >> 16) & 0xFFFF) as u16 },
                other => panic!("Unknown field '{}': expected a, b, c, or d", other),
            };

            quote! { #field_ident: #expr }
        });

        quote! { #i => Self::#ident { #(#decoded_fields),* }, }
    });

    quote! {
        impl #name {
            pub fn new(data: &mut impl Buf) -> Self {
                let insn = data.get_u32_ne();

                match insn & 0xFF {
                    #( #arms )*
                    _ => unimplemented!(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::insns::bytecode_insn_impl;

    use assert_tokens_eq::assert_tokens_eq;
    use quote::quote;

    #[test]
    pub fn test_codegen() {
        let source = quote! {
            pub enum Instruction {
                A { a: u8 },
                B { b: u8 },
                C { c: u8 },
                D { d: u16 },
            }
        };

        let output = bytecode_insn_impl(source);

        let expected = quote! {
            impl Instruction {
                pub fn new(data: &mut impl Buf) -> Self {
                    let insn = data.get_u32_ne();
                    match insn & 0xFF {
                        0u32 => Self::A { a: ((insn >> 8) & 0xFF) as u8 },
                        1u32 => Self::B { b: ((insn >> 16) & 0xFF) as u8 },
                        2u32 => Self::C { c: ((insn >> 24) & 0xFF) as u8 },
                        3u32 => Self::D { d: ((insn >> 16) & 0xFFFF) as u16 },
                        _ => unimplemented!(),
                    }
                }
            }
        };
        assert_tokens_eq!(output, expected)
    }
}
