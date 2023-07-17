import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import * as spl from "@solana/spl-token";

import { expect } from "chai";

import web3, { Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { Vault } from "../target/types/vault";

import {
  COMMITMENT,
  PDAAccounts,
  ParsedTokenTransfer,
  createMint,
  createTokenAccount,
  getPDAs,
} from "./utils";

describe("deposit", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const { connection } = provider;

  const program = anchor.workspace.Vault as Program<Vault>;

  it("deposits into the vault token account and updates deposited amount in the vault account", async () => {
    try {
      // const owner = Keypair.generate().publicKey;
      const owner = provider.wallet.publicKey;
      const mint = await createMint(provider);
      const ownerTokenAccount = await createTokenAccount(
        provider,
        provider.wallet.publicKey,
        mint,
        100_000 * LAMPORTS_PER_SOL
      );

      // params.grantTokenAmount = new anchor.BN(0);
      const { vault, vaultTokenAccount, vaultAuthority } = await getPDAs({
        owner,
        programId: program.programId,
        mint,
      });

      const initializeTransaction = await program.methods
        .initializeVault(new anchor.BN(10))
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

      const depositTransaction = await program.methods
        .deposit(new anchor.BN(5))
        .accounts({
          vault,
          owner,
          ownerTokenAccount,
          vaultAuthority,
          vaultTokenAccount,
          mint,
          // Uncomment here for triggering bug
          // tokenProgram: mint,
        })
        .rpc(COMMITMENT);
      console.log(`[deposit] ${depositTransaction}`);

      const tx = await connection.getParsedTransaction(
        depositTransaction,
        COMMITMENT
      );

      // Ensure that inner transfer succeded.
      const transferIx: any = tx.meta.innerInstructions[0].instructions.find(
        (ix) =>
          (ix as any).parsed.type === "transfer" &&
          ix.programId.toBase58() == spl.TOKEN_PROGRAM_ID.toBase58()
      );
      const parsedInfo: ParsedTokenTransfer = transferIx.parsed.info;
      expect(parsedInfo).eql({
        amount: "5",
        authority: owner.toBase58(),
        destination: vaultTokenAccount.toBase58(),
        source: ownerTokenAccount.toBase58(),
      });

      // Check data
      const vaultData = await program.account.vault.fetch(vault);
      console.log(vaultData);
      expect(vaultData.owner.toBase58()).to.eq(owner.toBase58());
      expect(vaultData.initialized).to.eq(true);

      expect(vaultData.depositedAmount.toNumber()).to.eq(15);
      expect(vaultData.mint.toBase58()).to.eql(mint.toBase58());
      expect(vaultData.bumps.vault).to.not.eql(0);
      expect(vaultData.bumps.vaultAuthority).to.not.eql(0);
      expect(vaultData.bumps.vaultTokenAccount).to.not.eql(0);
    } catch (error) {
      console.error(error);
      throw new Error(error);
    }
  });
});
