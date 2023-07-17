# Vault
This is an anchor solana program that allows wallets to create a USDC (or any SPL token) vault,
transfer tokens to it, and withdraw from it.
This repo also contains a rust service in the `rust-service` folder which is supposed to crawl 
the user vaults once per month and transfer 1% of the currently staked amount to the vault in interest.
There is also a `vaultcoin` folder which contains a script that creates an spl token and deposits the token 
to the vault program by calling the `initializeVault` instruction in the program.
# How to test the solana program
To test the solana program you can simply run `anchor test`
# How to test the rust service
* `cd` to the `vaultcoin` directory and install dependencies `npm install`
* Run the vaultcoin script `npm start`
* Copy the secret key saved in the created `.env` file into `~/.config/solana/vault.json`
* `cd` to the `rust-service` directory and run `cargo run`, you should see some logs of transaciton signatures
for the interest transfers.
