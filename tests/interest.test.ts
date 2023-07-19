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

describe("Interest", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const { connection } = provider;

  const program = anchor.workspace.Vault as Program<Vault>;

  it("sends interest into the vault token account and updates interest earned in the vault account", async () => {
    try {
      // const owner = Keypair.generate().publicKey;
      const owner = provider.wallet.publicKey;

      const senderKeypair = Keypair.generate();
      const sender = senderKeypair.publicKey;
      const mint = await createMint(provider);
      const ownerTokenAccount = await createTokenAccount(
        provider,
        provider.wallet.publicKey,
        mint,
        100_000 * LAMPORTS_PER_SOL
      );

      const senderTokenAccount = await createTokenAccount(
        provider,
        sender,
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
        .deposit(new anchor.BN(50_000 * LAMPORTS_PER_SOL))
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

      const interestTransaction = await program.methods
        .sendInterest()
        .accounts({
          vault,
          owner,
          ownerTokenAccount,
          sender,
          senderTokenAccount,
          vaultAuthority,
          vaultTokenAccount,
          mint,
          // Uncomment here for triggering bug
          // tokenProgram: mint,
        })
        .signers([senderKeypair])
        .rpc(COMMITMENT);
      console.log(`[interest] ${interestTransaction}`);

      const tx = await connection.getParsedTransaction(
        interestTransaction,
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
        amount: `${50_000 * LAMPORTS_PER_SOL * 0.01}`,
        authority: sender.toBase58(),
        destination: vaultTokenAccount.toBase58(),
        source: senderTokenAccount.toBase58(),
      });

      // Check data
      const vaultData = await program.account.vault.fetch(vault);
      console.log(vaultData);
      expect(vaultData.owner.toBase58()).to.eq(owner.toBase58());
      expect(vaultData.initialized).to.eq(true);

      expect(vaultData.depositedAmount.toNumber()).to.eq(
        50_000 * LAMPORTS_PER_SOL + 10
      );
      expect(vaultData.interestEarned.toNumber()).to.eq(
        50_000 * LAMPORTS_PER_SOL * 0.01
      );
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
