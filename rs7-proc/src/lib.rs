extern crate proc_macro;

use proc_macro::TokenStream;
use syn::DeriveInput;

mod insns;

#[proc_macro_derive(BytecodeInstruction)]
pub fn bytecode_insn(input: TokenStream) -> TokenStream {
    let result = insns::bytecode_insn_impl(proc_macro2::TokenStream::from(input));

    TokenStream::from(result)
}
