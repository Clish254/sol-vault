use std::borrow::Borrow;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, MemcmpEncoding, RpcFilterType},
};

use anchor_lang::{prelude::AnchorDeserialize, AnchorSerialize};
use solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use spl_token::state::Account as TokenAccount;
use vault::Vault;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up the Solana RPC client
    let rpc_client = RpcClient::new_with_commitment(
        "https://api.devnet.solana.com".to_string(),
        CommitmentConfig::processed(),
    );

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

    println!("vaults : {:?}", vaults);

    // Iterate through each vault and process them
    for (vault_address, vault_account) in vaults {
        let deserialized_data: Vault =
            AnchorDeserialize::try_from_slice(&mut vault_account.data.borrow())?;

        // get vault token account where we'll send interest
        let (vault_token_account_pda, bump) =
            Pubkey::find_program_address(&[b"token", vault_address.as_ref()], &vault_program_id);

        // transfer 1% of staked amount to the vault token account as interest
    }

    Ok(())
}
