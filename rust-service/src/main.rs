use std::borrow::Borrow;

use anchor_lang::AccountDeserialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::RpcFilterType,
};

use anchor_lang::prelude::AnchorDeserialize;
use solana_account_decoder::UiAccountEncoding;
use solana_program::program_pack::Pack;

use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::{Account as TokenAccount, Mint as MintAccount};
use spl_token::{instruction::transfer, ID as TOKEN_PROGRAM_ID};
use vault::Vault;

#[derive(Clone, Debug)]
pub struct VaultRecord(Vault);

impl AccountDeserialize for VaultRecord {
    fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        let mut data = buf;
        let vault_record: Vault = AnchorDeserialize::deserialize(&mut data)
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;
        Ok(VaultRecord(vault_record))
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        // let mut data = buf;
        let mut data: &[u8] = &buf[8..];
        let vault_record: Vault = AnchorDeserialize::deserialize(&mut data)
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;
        Ok(VaultRecord(vault_record))
    }
}

impl VaultRecord {
    pub fn mint(&self) -> &Pubkey {
        &self.0.mint
    }

    pub fn owner(&self) -> &Pubkey {
        &self.0.owner
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    disburse_interest().await?;
    Ok(())
}

async fn disburse_interest() -> Result<(), Box<dyn std::error::Error>> {
    // Set up the Solana RPC client
    let rpc_client = RpcClient::new_with_commitment(
        "https://api.devnet.solana.com".to_string(),
        CommitmentConfig::finalized(),
    );
    let payer = read_keypair_file(&*shellexpand::tilde("~/.config/solana/vault.json"))
        .expect("Example requires a keypair file");
    let vault_program_id: Pubkey = "DpLHaRUPhCru3F8f3Aa1V8xHAxKmb9cdEqFD3E9BHRXv".parse()?;

    let filters = Some(vec![
        // RpcFilterType::Memcmp(Memcmp::new(0, MemcmpEncodedBytes::Bytes(b"vault".to_vec()))),
        RpcFilterType::DataSize(Vault::LEN as u64),
    ]);

    // Fetch the list of vaults
    let vaults = rpc_client
        .get_program_accounts_with_config(
            &vault_program_id,
            RpcProgramAccountsConfig {
                filters,
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    data_slice: None,
                    commitment: Some(rpc_client.commitment()),
                    min_context_slot: None,
                },
                with_context: Some(false),
                // ..RpcProgramAccountsConfig::default()
            },
        )
        .await
        .unwrap();

    // Iterate through each vault and process them
    for (vault_address, vault_account) in vaults {
        let deserialized_data: VaultRecord =
            VaultRecord::try_deserialize_unchecked(&mut vault_account.data.borrow())?;
        let mint = deserialized_data.mint();

        // get vault token account where we'll send interest
        let (vault_token_account_pda, _bump) = Pubkey::find_program_address(
            &[b"tokens".as_ref(), vault_address.as_ref()],
            &vault_program_id,
        );

        let serialized_token_account_data = rpc_client
            .get_account_data(&vault_token_account_pda)
            .await
            .unwrap();

        let token_account_data =
            TokenAccount::unpack_from_slice(&serialized_token_account_data).unwrap();

        let serialized_mint_account_data = rpc_client.get_account_data(mint).await.unwrap();

        let mint_account_data =
            MintAccount::unpack_from_slice(&serialized_mint_account_data).unwrap();

        let payer_is_mint_authority =
            mint_account_data.freeze_authority == Some(payer.pubkey()).into();

        // transfer 1% of staked amount to the vault token account as interest
        if payer_is_mint_authority == true {
            let interest = 0.01 * token_account_data.amount as f64;

            let payer_associated_token_account =
                get_associated_token_address(&payer.pubkey(), &mint);
            let transfer_instruction = transfer(
                &TOKEN_PROGRAM_ID,
                &payer_associated_token_account,
                &vault_token_account_pda,
                &payer.pubkey(),
                &[&payer.pubkey()],
                interest.trunc() as u64,
            )
            .unwrap();

            let blockhash = rpc_client.get_latest_blockhash().await?;

            let tx = Transaction::new_signed_with_payer(
                &[transfer_instruction],
                Some(&payer.pubkey()),
                &[&payer],
                blockhash,
            );
            let sig = rpc_client.send_and_confirm_transaction(&tx).await;

            println!("Interest transfer signature {:?}", sig);
        }
    }

    Ok(())
}
