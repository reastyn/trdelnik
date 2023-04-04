// DO NOT EDIT - automatically generated file (except `use` statements inside the `*_instruction` module
pub mod turnstile_instruction {
    use trdelnik_client::*;
    pub static PROGRAM_ID: Pubkey = Pubkey::new_from_array([
        216u8, 55u8, 200u8, 93u8, 189u8, 81u8, 94u8, 109u8, 14u8, 249u8, 244u8, 106u8, 68u8, 214u8,
        222u8, 190u8, 9u8, 25u8, 199u8, 75u8, 79u8, 230u8, 94u8, 137u8, 51u8, 187u8, 193u8, 48u8,
        87u8, 222u8, 175u8, 163u8,
    ]);
    pub async fn initialize(
        client: &Client,
        parameters: turnstile::instruction::Initialize,
        accounts: turnstile::accounts::Initialize,
        signers: impl IntoIterator<Item = Keypair> + Send + 'static,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, ClientError> {
        Ok(client
            .send_instruction(PROGRAM_ID, parameters, accounts, signers)
            .await?)
    }
    pub fn initialize_ix(
        parameters: turnstile::instruction::Initialize,
        accounts: turnstile::accounts::Initialize,
    ) -> Instruction {
        Instruction {
            program_id: PROGRAM_ID,
            data: parameters.data(),
            accounts: accounts.to_account_metas(None),
        }
    }
    pub async fn coin(
        client: &Client,
        parameters: turnstile::instruction::Coin,
        accounts: turnstile::accounts::UpdateState,
        signers: impl IntoIterator<Item = Keypair> + Send + 'static,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, ClientError> {
        Ok(client
            .send_instruction(PROGRAM_ID, parameters, accounts, signers)
            .await?)
    }
    pub fn coin_ix(
        parameters: turnstile::instruction::Coin,
        accounts: turnstile::accounts::UpdateState,
    ) -> Instruction {
        Instruction {
            program_id: PROGRAM_ID,
            data: parameters.data(),
            accounts: accounts.to_account_metas(None),
        }
    }
    pub async fn push(
        client: &Client,
        parameters: turnstile::instruction::Push,
        accounts: turnstile::accounts::UpdateState,
        signers: impl IntoIterator<Item = Keypair> + Send + 'static,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, ClientError> {
        Ok(client
            .send_instruction(PROGRAM_ID, parameters, accounts, signers)
            .await?)
    }
    pub fn push_ix(
        parameters: turnstile::instruction::Push,
        accounts: turnstile::accounts::UpdateState,
    ) -> Instruction {
        Instruction {
            program_id: PROGRAM_ID,
            data: parameters.data(),
            accounts: accounts.to_account_metas(None),
        }
    }
}
