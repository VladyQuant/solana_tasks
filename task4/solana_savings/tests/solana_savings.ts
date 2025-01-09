import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolanaSavings } from "../target/types/solana_savings";
import { expect } from "chai";
import { PublicKey, Connection, LAMPORTS_PER_SOL } from "@solana/web3.js";

describe("solana_savings", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaSavings as Program<SolanaSavings>;

  let depositAccount: anchor.web3.Keypair;

  it('Initializes the contract', async () => {
      depositAccount = anchor.web3.Keypair.generate();

      const tx = await program.methods.initialize()
          .accountsStrict({
              depositAccount: depositAccount.publicKey,
              user: provider.wallet.publicKey,
              systemProgram: anchor.web3.SystemProgram.programId,
          })
          .signers([depositAccount])
          .rpc();
      console.log('Initialize transaction signature', tx);

      const account = await program.account.depositAccount.fetch(depositAccount.publicKey);
      console.log('Total Deposits:', account.totalDeposits.toString());
      expect(account.totalDeposits.toNumber()).equal(0);
  });

  it('Handles multiple deposits and withdrawal', async () => {
    const depositAmount = new anchor.BN(100000000); // 0.1 SOL

    // First Deposit
    await program.methods.deposit(depositAmount).accounts({
        depositAccount: depositAccount.publicKey,
        user: provider.wallet.publicKey,
    }).rpc();

    // Second Deposit
    await program.methods.deposit(depositAmount).accounts({
        depositAccount: depositAccount.publicKey,
        user: provider.wallet.publicKey,
    }).rpc();

    // Withdraw
    await program.methods.withdraw(depositAmount).accounts({
        depositAccount: depositAccount.publicKey,
        user: provider.wallet.publicKey,
    }).rpc();

    // Fetch the account and check deposits
    const account = await program.account.depositAccount.fetch(depositAccount.publicKey);
    console.log('Total Deposits after deposit and withdraw:', account.totalDeposits.toString());

    const expectedRemainingDeposit = new anchor.BN(100000000); // Expected remaining deposit after withdrawal
    expect(account.totalDeposits.eq(expectedRemainingDeposit)).to.be.true; // Using BN.eq for comparison
  });

  it('Handles get balance', async () => {
    const balance = await program.methods.getBalance()
      .accounts({
        depositAccount: depositAccount.publicKey,
        user: provider.wallet.publicKey,
      })
      .view();
    expect(balance.eq(100000000));
  })
})