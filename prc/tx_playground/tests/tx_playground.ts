import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  getAccount,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";
import { TxPlayground } from "../target/types/tx_playground";

const AGGREGATOR_NAME_LEN = 16;
const TX_LOG_SEED_PREFIX = Buffer.from("tx_log");
const USER_STATE_SEED_PREFIX = Buffer.from("user_state");
const CONFIG_SEED_PREFIX = Buffer.from("config");

const ZERO_AGGREGATOR = Array(AGGREGATOR_NAME_LEN).fill(0);

function toLEBytes(value: number): Buffer {
  const buf = Buffer.alloc(8);
  buf.writeBigUInt64LE(BigInt(value));
  return buf;
}

function makeAggregatorBytes(label: string): number[] {
  const buf = Buffer.alloc(AGGREGATOR_NAME_LEN);
  buf.write(label);
  return Array.from(buf);
}

function unwrapAggregator(raw: any): number[] {
  if (Array.isArray(raw)) {
    return raw as number[];
  }
  if (raw instanceof Uint8Array) {
    return Array.from(raw);
  }
  if (raw && typeof raw === "object") {
    if (Array.isArray(raw.data)) {
      return raw.data as number[];
    }
  }
  throw new Error("Unable to decode aggregator representation");
}

describe("tx_playground", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.txPlayground as Program<TxPlayground>;
  const wallet = provider.wallet;

  const aggregatorManual = { data: makeAggregatorBytes("manual") };
  const aggregatorRoute = { data: makeAggregatorBytes("router") };

  let configPda: PublicKey;
  let userStatePda: PublicKey;
  let userTokenAccount: PublicKey;
  let destinationTokenAccount: PublicKey;
  let mint: PublicKey;

  const destinationOwner = anchor.web3.Keypair.generate();

  before("set up test environment", async () => {
    [configPda] = PublicKey.findProgramAddressSync(
      [CONFIG_SEED_PREFIX],
      program.programId
    );

    [userStatePda] = PublicKey.findProgramAddressSync(
      [USER_STATE_SEED_PREFIX, wallet.publicKey.toBuffer()],
      program.programId
    );

    const airdropTx = await provider.connection.requestAirdrop(
      destinationOwner.publicKey,
      1 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(airdropTx);

    mint = await createMint(
      provider.connection,
      wallet.payer,
      wallet.publicKey,
      null,
      6
    );

    ({ address: userTokenAccount } = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      wallet.payer,
      mint,
      wallet.publicKey
    ));

    ({ address: destinationTokenAccount } =
      await getOrCreateAssociatedTokenAccount(
        provider.connection,
        wallet.payer,
        mint,
        destinationOwner.publicKey
      ));

    await mintTo(
      provider.connection,
      wallet.payer,
      mint,
      userTokenAccount,
      wallet.publicKey,
      1_000_000_000
    );
  });

  it("initializes config with aggregators", async () => {
    await program.methods
      .initializeConfig([aggregatorManual, aggregatorRoute])
      .accounts({
        authority: wallet.publicKey,
        config: configPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const configAccount = await program.account.config.fetch(configPda);
    console.log("configAccount", {
      owner: configAccount.owner.toBase58(),
      aggregatorCount: configAccount.aggregatorCount,
      aggregators: configAccount.aggregators.map((agg) =>
        unwrapAggregator(agg)
      ),
    });
    expect(configAccount.owner.toBase58()).to.equal(
      wallet.publicKey.toBase58()
    );
    expect(configAccount.aggregatorCount).to.equal(2);
    expect(unwrapAggregator(configAccount.aggregators[0])).to.deep.equal(
      aggregatorManual.data
    );
    expect(unwrapAggregator(configAccount.aggregators[1])).to.deep.equal(
      aggregatorRoute.data
    );
  });

  it("initializes user state", async () => {
    await program.methods
      .initializeUserState()
      .accounts({
        authority: wallet.publicKey,
        config: configPda,
        userState: userStatePda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const userState = await program.account.userState.fetch(userStatePda);
    console.log("userState after init", {
      lastTxId: userState.lastTxId.toNumber(),
      bump: userState.bump,
    });
    expect(userState.lastTxId.toNumber()).to.equal(0);
  });

  it("performs transfer and records log", async () => {
    const logId = 1;
    const logSeed = toLEBytes(logId);
    const [txLogPda] = PublicKey.findProgramAddressSync(
      [TX_LOG_SEED_PREFIX, wallet.publicKey.toBuffer(), logSeed],
      program.programId
    );
    console.log("derived tx_log", txLogPda.toBase58());

    const amount = new BN(100_000_000);

    const transferBuilder = program.methods
      .transfer(amount, Array.from(logSeed))
      .accounts({
        authority: wallet.publicKey,
        config: configPda,
        userState: userStatePda,
        userSource: userTokenAccount,
        destinationToken: destinationTokenAccount,
        txLog: txLogPda,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      });

    const ix = await transferBuilder.instruction();
    console.log(
      "instruction accounts",
      ix.keys.map((k) => k.pubkey.toBase58())
    );

    const userStateBefore = await program.account.userState.fetch(userStatePda);
    const userBalanceBefore = await getAccount(
      provider.connection,
      userTokenAccount
    );
    const destinationBalanceBefore = await getAccount(
      provider.connection,
      destinationTokenAccount
    );
    console.log("transfer before", {
      userStateLastTxId: userStateBefore.lastTxId.toNumber(),
      userBalance: Number(userBalanceBefore.amount),
      destinationBalance: Number(destinationBalanceBefore.amount),
    });

    await provider.sendAndConfirm(new anchor.web3.Transaction().add(ix));

    const userState = await program.account.userState.fetch(userStatePda);
    expect(userState.lastTxId.toNumber()).to.equal(logId);

    const logAccount = await program.account.txLog.fetch(txLogPda);
    console.log("transfer txLog", {
      mode: logAccount.mode,
      amountIn: logAccount.amountIn.toString(),
      amountOut: logAccount.amountOut.toString(),
      timestamp: logAccount.timestamp.toString(),
    });
    expect(logAccount.mode).to.deep.equal({ transfer: {} });
    expect(logAccount.amountIn.toNumber()).to.equal(amount.toNumber());
    expect(logAccount.amountOut.toNumber()).to.equal(amount.toNumber());
    expect(unwrapAggregator(logAccount.aggregator)).to.deep.equal(
      ZERO_AGGREGATOR
    );

    const userBalance = await getAccount(provider.connection, userTokenAccount);
    const destinationBalance = await getAccount(
      provider.connection,
      destinationTokenAccount
    );

    expect(Number(userBalance.amount)).to.equal(900_000_000);
    expect(Number(destinationBalance.amount)).to.equal(100_000_000);
    console.log("transfer after", {
      userStateLastTxId: userState.lastTxId.toNumber(),
      userBalance: Number(userBalance.amount),
      destinationBalance: Number(destinationBalance.amount),
    });
  });

  it("executes manual swap and logs result", async () => {
    const logId = 2;
    const logSeed = toLEBytes(logId);
    const [txLogPda] = PublicKey.findProgramAddressSync(
      [TX_LOG_SEED_PREFIX, wallet.publicKey.toBuffer(), logSeed],
      program.programId
    );

    const amountIn = new BN(50_000_000);
    const expectedOut = new BN(48_000_000);

    const before = await program.account.userState.fetch(userStatePda);
    console.log("manual swap before", {
      lastTxId: before.lastTxId.toNumber(),
    });

    await program.methods
      .manualSwap(amountIn, expectedOut, Array.from(logSeed))
      .accounts({
        authority: wallet.publicKey,
        config: configPda,
        userState: userStatePda,
        userSource: userTokenAccount,
        destinationToken: destinationTokenAccount,
        txLog: txLogPda,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const userState = await program.account.userState.fetch(userStatePda);
    expect(userState.lastTxId.toNumber()).to.equal(logId);

    const logAccount = await program.account.txLog.fetch(txLogPda);
    console.log("manual swap txLog", {
      mode: logAccount.mode,
      amountIn: logAccount.amountIn.toString(),
      amountOut: logAccount.amountOut.toString(),
      timestamp: logAccount.timestamp.toString(),
    });
    expect(logAccount.mode).to.deep.equal({ manualSwap: {} });
    expect(logAccount.amountIn.toNumber()).to.equal(amountIn.toNumber());
    expect(logAccount.amountOut.toNumber()).to.equal(expectedOut.toNumber());
    expect(unwrapAggregator(logAccount.aggregator)).to.deep.equal(
      ZERO_AGGREGATOR
    );
    console.log("manual swap after", {
      lastTxId: userState.lastTxId.toNumber(),
    });
  });

  it("executes aggregator swap and logs aggregator name", async () => {
    const logId = 3;
    const logSeed = toLEBytes(logId);
    const [txLogPda] = PublicKey.findProgramAddressSync(
      [TX_LOG_SEED_PREFIX, wallet.publicKey.toBuffer(), logSeed],
      program.programId
    );

    const amountIn = new BN(25_000_000);
    const minAmountOut = new BN(24_000_000);

    const before = await program.account.userState.fetch(userStatePda);
    console.log("aggregator swap before", {
      lastTxId: before.lastTxId.toNumber(),
    });

    await program.methods
      .aggregatorSwap(
        aggregatorRoute,
        amountIn,
        minAmountOut,
        Array.from(logSeed)
      )
      .accounts({
        authority: wallet.publicKey,
        config: configPda,
        userState: userStatePda,
        userSource: userTokenAccount,
        destinationToken: destinationTokenAccount,
        txLog: txLogPda,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const userState = await program.account.userState.fetch(userStatePda);
    expect(userState.lastTxId.toNumber()).to.equal(logId);

    const logAccount = await program.account.txLog.fetch(txLogPda);
    console.log("aggregator swap txLog", {
      mode: logAccount.mode,
      amountIn: logAccount.amountIn.toString(),
      amountOut: logAccount.amountOut.toString(),
      aggregator: unwrapAggregator(logAccount.aggregator),
      timestamp: logAccount.timestamp.toString(),
    });
    expect(logAccount.mode).to.deep.equal({ aggregatorSwap: {} });
    expect(unwrapAggregator(logAccount.aggregator)).to.deep.equal(
      aggregatorRoute.data
    );
    expect(logAccount.amountIn.toNumber()).to.equal(amountIn.toNumber());
    expect(logAccount.amountOut.toNumber()).to.equal(minAmountOut.toNumber());
    console.log("aggregator swap after", {
      lastTxId: userState.lastTxId.toNumber(),
    });
  });
});
