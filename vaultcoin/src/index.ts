import { initializeKeypair, airdropSolIfNeeded } from "./initializeKeypair";
import * as web3 from "@solana/web3.js";
import * as token from "@solana/spl-token";
import * as anchor from "@coral-xyz/anchor";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import {
  Metaplex,
  keypairIdentity,
  bundlrStorage,
  toMetaplexFile,
} from "@metaplex-foundation/js";
import {
  DataV2,
  createCreateMetadataAccountV2Instruction,
  createUpdateMetadataAccountV2Instruction,
  createCreateMetadataAccountV3Instruction,
} from "@metaplex-foundation/mpl-token-metadata";
import * as fs from "fs";
import vaultIDL from "../../target/idl/vault.json";

async function createNewMint(
  connection: web3.Connection,
  payer: web3.Keypair,
  mintAuthority: web3.PublicKey,
  freezeAuthority: web3.PublicKey,
  decimals: number,
  keypair: web3.Keypair,
  confirmOptions: web3.ConfirmOptions
): Promise<web3.PublicKey> {
  const tokenMint = await token.createMint(
    connection,
    payer,
    mintAuthority,
    freezeAuthority,
    decimals,
    keypair,
    confirmOptions
  );

  console.log(`The token mint account address is ${tokenMint}`);
  console.log(
    `Token Mint: https://explorer.solana.com/address/${tokenMint}?cluster=devnet`
  );

  return tokenMint;
}

async function createTokenAccount(
  connection: web3.Connection,
  payer: web3.Keypair,
  mint: web3.PublicKey,
  owner: web3.PublicKey
) {
  const tokenAccount = await token.getOrCreateAssociatedTokenAccount(
    connection,
    payer,
    mint,
    owner
  );

  console.log(
    `Token Account: https://explorer.solana.com/address/${tokenAccount.address}?cluster=devnet`
  );

  return tokenAccount;
}

async function mintTokens(
  connection: web3.Connection,
  payer: web3.Keypair,
  mint: web3.PublicKey,
  destination: web3.PublicKey,
  authority: web3.Keypair,
  amount: number
) {
  const mintInfo = await token.getMint(connection, mint);

  const transactionSignature = await token.mintTo(
    connection,
    payer,
    mint,
    destination,
    authority,
    amount * 10 ** mintInfo.decimals
  );

  console.log(
    `Mint Token Transaction: https://explorer.solana.com/tx/${transactionSignature}?cluster=devnet`
  );
}

async function transferTokens(
  connection: web3.Connection,
  payer: web3.Keypair,
  source: web3.PublicKey,
  destination: web3.PublicKey,
  owner: web3.PublicKey,
  amount: number,
  mint: web3.PublicKey
) {
  const mintInfo = await token.getMint(connection, mint);

  const transactionSignature = await token.transfer(
    connection,
    payer,
    source,
    destination,
    owner,
    amount * 10 ** mintInfo.decimals
  );

  console.log(
    `Transfer Transaction: https://explorer.solana.com/tx/${transactionSignature}?cluster=devnet`
  );
}

async function createTokenMetadata(
  connection: web3.Connection,
  metaplex: Metaplex,
  mint: web3.PublicKey,
  user: web3.Keypair,
  name: string,
  symbol: string,
  description: string
) {
  // file to buffer
  const buffer = fs.readFileSync("assets/work.png");

  // buffer to metaplex file
  const file = toMetaplexFile(buffer, "work.png");

  // upload image and get image uri
  const imageUri = await metaplex.storage().upload(file);
  console.log("image uri:", imageUri);

  // upload metadata and get metadata uri (off chain metadata)
  const { uri } = await metaplex.nfts().uploadMetadata({
    name: name,
    description: description,
    image: imageUri,
  });

  console.log("metadata uri:", uri);

  // get metadata account address
  const metadataPDA = metaplex.nfts().pdas().metadata({ mint });

  // onchain metadata format
  const tokenMetadata = {
    name: name,
    symbol: symbol,
    uri: uri,
    sellerFeeBasisPoints: 0,
    creators: null,
    collection: null,
    uses: null,
  } as DataV2;

  // transaction to create metadata account
  const transaction = new web3.Transaction().add(
    createCreateMetadataAccountV3Instruction(
      {
        metadata: metadataPDA,
        mint: mint,
        mintAuthority: user.publicKey,
        payer: user.publicKey,
        updateAuthority: user.publicKey,
      },
      {
        createMetadataAccountArgsV3: {
          data: tokenMetadata,
          isMutable: true,
          collectionDetails: null,
        },
      }
    )
  );

  // send transaction
  const transactionSignature = await web3.sendAndConfirmTransaction(
    connection,
    transaction,
    [user]
  );

  console.log(
    `Create Metadata Account: https://explorer.solana.com/tx/${transactionSignature}?cluster=devnet`
  );
}

export const COMMITMENT: { commitment: web3.Finality } = {
  commitment: "confirmed",
};

export interface PDAAccounts {
  vault: web3.PublicKey;
  vaultAuthority: web3.PublicKey;
  vaultTokenAccount: web3.PublicKey;
}

export interface ParsedTokenTransfer {
  amount: string;
  authority: string;
  destination: string;
  source: string;
}

export const getPDAs = async (params: {
  programId: web3.PublicKey;
  owner: web3.PublicKey;
  mint: web3.PublicKey;
}): Promise<PDAAccounts> => {
  const [vault] = await web3.PublicKey.findProgramAddress(
    [Buffer.from("vault"), params.owner.toBuffer(), params.mint.toBuffer()],
    params.programId
  );
  const [vaultAuthority] = await web3.PublicKey.findProgramAddress(
    [Buffer.from("authority"), vault.toBuffer()],
    params.programId
  );
  const [vaultTokenAccount] = await web3.PublicKey.findProgramAddress(
    [Buffer.from("tokens"), vault.toBuffer()],
    params.programId
  );

  return {
    vault,
    vaultAuthority,
    vaultTokenAccount,
  };
};

async function initializeVault(
  user: web3.Keypair,
  mint: web3.PublicKey,
  user_token_account: web3.PublicKey,
  connection: web3.Connection
) {
  try {
    let wallet = new NodeWallet(user);
    const provider = new anchor.AnchorProvider(
      connection,
      wallet,
      anchor.AnchorProvider.defaultOptions()
    );

    const programId = vaultIDL.metadata.address;
    const program = new anchor.Program(
      vaultIDL as anchor.Idl,
      programId,
      provider
    );
    const owner = user.publicKey;
    const ownerTokenAccount = user_token_account;
    // params.grantTokenAmount = new anchor.BN(0);
    const { vault, vaultTokenAccount, vaultAuthority } = await getPDAs({
      owner,
      programId: program.programId,
      mint,
    });

    const initializeTransaction = await program.methods
      .initializeVault(new anchor.BN(400))
      .accounts({
        vault,
        owner,
        mint,
        ownerTokenAccount,
        vaultAuthority,
        vaultTokenAccount,
        // Uncomment here for triggering bug
        // tokenProgram: mint,
      })
      .rpc(COMMITMENT);
    console.log(`[Initialize] ${initializeTransaction}`);

    const vaultData = await program.account.vault.fetch(vault);
    console.log(vaultData);
  } catch (error) {
    console.error(error);
  }
}

async function main() {
  const connection = new web3.Connection(web3.clusterApiUrl("devnet"), {
    commitment: "confirmed",
  });
  const user = await initializeKeypair(connection);

  console.log("PublicKey:", user.publicKey.toBase58());

  const mint = await createNewMint(
    connection,
    user, // We'll pay the fees
    user.publicKey, // We're the mint authority
    user.publicKey, // And the freeze authority >:)
    2, // Only two decimals!
    web3.Keypair.generate(),
    { commitment: "finalized" }
  );

  const tokenAccount = await createTokenAccount(
    connection,
    user,
    mint,
    user.publicKey // Associating our address with the token account
  );

  // Mint 1000 tokens to our address
  await mintTokens(connection, user, mint, tokenAccount.address, user, 1000);

  const receiver = web3.Keypair.generate();

  const receiverTokenAccount = await createTokenAccount(
    connection,
    user,
    mint,
    receiver.publicKey
  );

  await transferTokens(
    connection,
    user,
    tokenAccount.address,
    receiverTokenAccount.address,
    user.publicKey,
    500,
    mint
  );

  await airdropSolIfNeeded(receiver, connection);

  const MINT_ADDRESS = mint;

  // metaplex setup
  const metaplex = Metaplex.make(connection)
    .use(keypairIdentity(user))
    .use(
      bundlrStorage({
        address: "https://devnet.bundlr.network",
        providerUrl: "https://api.devnet.solana.com",
        timeout: 60000,
      })
    );

  // Calling the token
  await createTokenMetadata(
    connection,
    metaplex,
    new web3.PublicKey(MINT_ADDRESS),
    user,
    "Vault",
    "VAULT",
    "Vault"
  );

  await initializeVault(
    receiver,
    mint,
    receiverTokenAccount.address,
    connection
  );
}

main()
  .then(() => {
    console.log("Finished successfully");
    process.exit(0);
  })
  .catch((error) => {
    console.log(error);
    process.exit(1);
  });
