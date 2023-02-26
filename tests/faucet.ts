import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { createAssociatedTokenAccount, createMint, mintTo } from "@solana/spl-token";
import { Faucet } from "../target/types/faucet";
import { PublicKey } from "@solana/web3.js";
import { expect } from "chai";

const delay = (seconds: number) => new Promise(resolve => setTimeout(resolve, seconds * 1000))

describe("faucet", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Faucet as Program<Faucet>;
  const connection = program.provider.connection;

  it("will do the success path", async () => {
    // Add your test here.
    const provider = program.provider as anchor.AnchorProvider;
    const wallet = provider.wallet as NodeWallet;
    const mint = await createMint(connection, wallet.payer, provider.publicKey, provider.publicKey, 9);
    const [faucet] = PublicKey.findProgramAddressSync(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("faucet")),
        mint.toBuffer(),
      ],
      program.programId
    );
    const [escrowWallet] = PublicKey.findProgramAddressSync(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("wallet")),
        mint.toBuffer(),
      ],
      program.programId
    );
    const tx = await program.methods.initialize(mint, new anchor.BN(10), new anchor.BN(1))
      .accounts({
        mint,
        faucet,
        escrowWallet,
      })
      .rpc();
    console.log("Your initialization transaction signature", tx);

    const [lastDrop] = PublicKey.findProgramAddressSync(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("last_drop")),
        wallet.publicKey.toBuffer(),
      ],
      program.programId
    );

    await mintTo(connection, wallet.payer, mint, escrowWallet, wallet.payer, 5_000_000_000);

    const toAccount = await createAssociatedTokenAccount(connection, wallet.payer, mint, wallet.publicKey)
    await program.methods.firstAirdrop()
      .accounts({
        mint,
        faucet,
        escrowWallet,
        lastDrop,
        to: toAccount,
      })
      .rpc();

    const error = await program.methods.airdrop()
      .accounts({
        mint,
        faucet,
        escrowWallet,
        lastDrop,
        to: toAccount,
      })
      .rpc()
      .catch(e => e);
    expect(error).to.be.instanceOf(Error)
    expect(error.message).to.contains('Wait for a while');

    console.log('waiting...')
    await delay(2);
    console.log('try again')
    await program.methods.airdrop()
      .accounts({
        mint,
        faucet,
        escrowWallet,
        lastDrop,
        to: toAccount,
      })
      .rpc();
  });
});
