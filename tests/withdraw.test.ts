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

describe("withdraw", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const { connection } = provider;

  const program = anchor.workspace.Vault as Program<Vault>;

  it("withdraws from the vault token account to the vault owner account and updates withdrawnAmount amount in the vault account", async () => {
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

      const withdrawTransaction = await program.methods
        .withdraw(new anchor.BN(5))
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
      console.log(`[deposit] ${withdrawTransaction}`);

      const tx = await connection.getParsedTransaction(
        withdrawTransaction,
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
        authority: vaultAuthority.toBase58(),
        destination: ownerTokenAccount.toBase58(),
        source: vaultTokenAccount.toBase58(),
      });

      // Check data
      const vaultData = await program.account.vault.fetch(vault);

      console.log(vaultData);
      expect(vaultData.owner.toBase58()).to.eq(owner.toBase58());
      expect(vaultData.initialized).to.eq(true);

      expect(vaultData.depositedAmount.toNumber()).to.eq(10);
      expect(vaultData.withdrawnAmount.toNumber()).to.eq(5);
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
