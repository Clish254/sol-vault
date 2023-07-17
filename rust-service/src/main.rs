use std::borrow::Borrow;

use anchor_lang::AccountDeserialize;
use solana_account_decoder::parse_token::get_token_account_mint;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, MemcmpEncoding, RpcFilterType},
};

use anchor_lang::{prelude::AnchorDeserialize, AnchorSerialize};
use solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig};
use solana_program::{
    program::invoke_signed,
    program_pack::{IsInitialized, Pack},
};

use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::{Account as TokenAccount, Mint as MintAccount};
use spl_token::{instruction::transfer, ID as TOKEN_PROGRAM_ID};
use vault::Vault;

use borsh::{BorshDeserialize, BorshSerialize};

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
// impl anchor_lang::Owner for VaultRecord {
//     fn owner() -> Pubkey {
//         let vault_program_id: Pubkey = "DpLHaRUPhCru3F8f3Aa1V8xHAxKmb9cdEqFD3E9BHRXv"
//             .parse()
//             .unwrap();
//         vault_program_id
//     }
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up the Solana RPC client
    let rpc_client = RpcClient::new_with_commitment(
        "https://api.devnet.solana.com".to_string(),
        CommitmentConfig::processed(),
    );
    let payer = read_keypair_file(&*shellexpand::tilde("~/.config/solana/id.json"))
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
        println!("vault address {:?}", vault_address);
        println!("vault data {:?}", deserialized_data);
        let mint = deserialized_data.mint();
        println!("vault mint {}", &mint);

        // break;
        // get vault token account where we'll send interest
        let (vault_token_account_pda, bump) = Pubkey::find_program_address(
            &[b"tokens".as_ref(), vault_address.as_ref()],
            &vault_program_id,
        );
        println!("vault token account {}", vault_token_account_pda);

        let serialized_token_account_data = rpc_client
            .get_account_data(&vault_token_account_pda)
            .await
            .unwrap();

        let token_account_data =
            TokenAccount::unpack_from_slice(&serialized_token_account_data).unwrap();

        println!("vault token account data {:?}", token_account_data);
        println!("vault token account amount {:?}", token_account_data.amount);

        println!("mint account data {:?}", token_account_data);

        // transfer 1% of staked amount to the vault token account as interest
        let interest = 1 / 100 * &token_account_data.amount;

        let payer_associated_token_account = get_associated_token_address(&payer.pubkey(), &mint);
        let vault_owner_associated_token_account =
            get_associated_token_address(&deserialized_data.owner(), &mint);
        let transfer_instruction = transfer(
            &TOKEN_PROGRAM_ID,
            &payer_associated_token_account,
            &vault_owner_associated_token_account,
            &payer.pubkey(),
            &[&payer.pubkey()],
            interest,
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

    Ok(())
}
