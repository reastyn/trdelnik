use crate::idl::Idl;
use quote::{format_ident, ToTokens};
use syn::{parse_quote, parse_str};

/// Generates `program_client`'s `lib.rs` from [Idl] created from Anchor programs.
/// Disable regenerating the `use` statements with a used imports `use_modules`
///
/// _Note_: See the crate's tests for output example.
pub fn generate_source_code(idl: Idl, use_modules: &[syn::ItemUse]) -> String {
    let mut output = "// DO NOT EDIT - automatically generated file (except `use` statements inside the `*_instruction` module\n".to_owned();
    let code = idl
        .programs
        .into_iter()
        .map(|idl_program| {
            let program_name = idl_program.name.snake_case.replace('-', "_");
            let instruction_module_name = format_ident!("{}_instruction", program_name);
            let module_name: syn::Ident = parse_str(&program_name).unwrap();
            let pubkey_bytes: syn::ExprArray = parse_str(&idl_program.id).unwrap();

            let instructions = idl_program
                .instruction_account_pairs
                .into_iter()
                .fold(
                    Vec::new(),
                    |mut instructions, (idl_instruction, idl_account_group)| {
                        let instruction_fn_name: syn::Ident =
                            parse_str(&idl_instruction.name.snake_case).unwrap();
                        let instruction_struct_name: syn::Ident =
                            parse_str(&idl_instruction.name.upper_camel_case).unwrap();
                        let account_struct_name: syn::Ident =
                            parse_str(&idl_account_group.name.upper_camel_case).unwrap();
                        let instruction_name: syn::Ident =
                            parse_str(&(idl_instruction.name.snake_case + "_ix")).unwrap();

                        let instruction: syn::ItemFn = parse_quote! {
                            pub async fn #instruction_fn_name(
                                client: &Client,
                                parameters: #module_name::instruction::#instruction_struct_name,
                                accounts: #module_name::accounts::#account_struct_name,
                                signers: impl IntoIterator<Item = Keypair> + Send + 'static,
                            ) -> Result<EncodedConfirmedTransactionWithStatusMeta, ClientError> {
                                Ok(client.send_instruction(
                                    PROGRAM_ID,
                                    parameters,
                                    accounts,
                                    signers,
                                ).await?)
                            }
                        };

                        let instruction_raw: syn::ItemFn = parse_quote! {
                            pub fn #instruction_name(
                                parameters: #module_name::instruction::#instruction_struct_name,
                                accounts: #module_name::accounts::#account_struct_name,
                            ) -> Instruction {
                                Instruction{
                                    program_id: PROGRAM_ID,
                                    data: parameters.data(),
                                    accounts: accounts.to_account_metas(None),
                                }
                            }
                        };

                        instructions.push(instruction);
                        instructions.push(instruction_raw);
                        instructions
                    },
                )
                .into_iter();

            let program_module: syn::ItemMod = parse_quote! {
                pub mod #instruction_module_name {
                    #(#use_modules)*
                    pub static PROGRAM_ID: Pubkey = Pubkey::new_from_array(#pubkey_bytes);
                    #(#instructions)*
                }
            };
            program_module.into_token_stream().to_string()
        })
        .collect::<String>();
    output.push_str(&code);
    output
}
