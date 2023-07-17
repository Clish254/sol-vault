# Vault
This is an anchor smart contract that allows wallets to create a USDC (or any SPL token) vault, transfer tokens to it, and withdraw from it. 
This repo also contains a rust service in the `rust-service` folder which is supposed to crawl the user vaults once per month and transfers 1% of the currently staked amount to the vault in interest.
