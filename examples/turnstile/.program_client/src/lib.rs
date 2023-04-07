// DO NOT EDIT - automatically generated file (except `use` statements inside the `*_instruction` module
pub mod turnstile_instruction {
    use trdelnik_client::*;
    pub static PROGRAM_ID: Pubkey = Pubkey::new_from_array([
        5u8, 214u8, 204u8, 101u8, 166u8, 163u8, 239u8, 244u8, 13u8, 110u8, 64u8, 106u8, 230u8,
        81u8, 141u8, 186u8, 208u8, 155u8, 78u8, 83u8, 194u8, 215u8, 103u8, 17u8, 94u8, 15u8, 137u8,
        68u8, 170u8, 153u8, 74u8, 59u8,
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
