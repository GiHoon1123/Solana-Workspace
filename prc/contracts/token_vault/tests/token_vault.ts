import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import {
  createMint,
  getAccount,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";
import { assert } from "chai";
import { TokenVault } from "../target/types/token_vault";

describe("token_vault", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.tokenVault as Program<TokenVault>;
  const payer = provider.wallet;

  // 테스트용 사용자들
  const tom = anchor.web3.Keypair.generate();
  const dustin = anchor.web3.Keypair.generate();

  let mint: PublicKey; // SPL 토큰 Mint
  let vaultPda: PublicKey; // Vault PDA
  let vaultTokenAccount: PublicKey; // Vault의 Token Account

  let tomTokenAccount: PublicKey; // Tom의 Token Account
  let dustinTokenAccount: PublicKey; // Dustin의 Token Account

  let tomUserVault: PublicKey; // Tom의 UserVault PDA
  let dustinUserVault: PublicKey; // Dustin의 UserVault PDA

  before(async () => {
    console.log("\n=== 테스트 환경 설정 ===\n");

    // 1. Tom과 Dustin에게 SOL 에어드랍 (수수료용)
    const tomAirdrop = await provider.connection.requestAirdrop(
      tom.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(tomAirdrop);
    console.log("Tom에게 2 SOL 에어드랍 완료");

    const dustinAirdrop = await provider.connection.requestAirdrop(
      dustin.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(dustinAirdrop);
    console.log("Dustin에게 2 SOL 에어드랍 완료");

    // 2. SPL 토큰 Mint 생성 (소수점 6자리)
    mint = await createMint(
      provider.connection,
      payer.payer,
      payer.publicKey,
      null,
      6 // decimals
    );
    console.log("SPL 토큰 Mint 생성:", mint.toString());

    // 3. Vault PDA 계산
    [vaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault")],
      program.programId
    );
    console.log("Vault PDA:", vaultPda.toString());

    // 4. Vault의 Token Account 생성 (Vault PDA가 owner)
    const vaultTokenAccountInfo = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer.payer,
      mint,
      vaultPda,
      true // allowOwnerOffCurve (PDA는 off-curve)
    );
    vaultTokenAccount = vaultTokenAccountInfo.address;
    console.log("Vault Token Account:", vaultTokenAccount.toString());

    // 5. Tom의 Token Account 생성
    const tomTokenAccountInfo = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer.payer,
      mint,
      tom.publicKey
    );
    tomTokenAccount = tomTokenAccountInfo.address;
    console.log("Tom Token Account:", tomTokenAccount.toString());

    // 6. Dustin의 Token Account 생성
    const dustinTokenAccountInfo = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer.payer,
      mint,
      dustin.publicKey
    );
    dustinTokenAccount = dustinTokenAccountInfo.address;
    console.log("Dustin Token Account:", dustinTokenAccount.toString());

    // 7. Tom에게 1000 토큰 발행
    await mintTo(
      provider.connection,
      payer.payer,
      mint,
      tomTokenAccount,
      payer.publicKey,
      1000 * 1_000_000 // 1000 토큰 (decimals=6)
    );
    console.log("Tom에게 1000 토큰 발행 완료");

    // 8. Dustin에게 500 토큰 발행
    await mintTo(
      provider.connection,
      payer.payer,
      mint,
      dustinTokenAccount,
      payer.publicKey,
      500 * 1_000_000 // 500 토큰
    );
    console.log("Dustin에게 500 토큰 발행 완료");

    // 9. UserVault PDA 계산
    [tomUserVault] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("user-vault"),
        tom.publicKey.toBuffer(),
        vaultPda.toBuffer(),
      ],
      program.programId
    );
    console.log("Tom UserVault PDA:", tomUserVault.toString());

    [dustinUserVault] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("user-vault"),
        dustin.publicKey.toBuffer(),
        vaultPda.toBuffer(),
      ],
      program.programId
    );
    console.log("Dustin UserVault PDA:", dustinUserVault.toString());

    console.log("\n=== 환경 설정 완료 ===\n");
  });

  it("Vault를 초기화한다", async () => {
    const tx = await program.methods
      .initializeVault()
      .accounts({
        owner: payer.publicKey,
        vault: vaultPda,
        vaultTokenAccount: vaultTokenAccount,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("Vault 초기화 트랜잭션:", tx);

    const vaultAccount = await program.account.vault.fetch(vaultPda);
    assert.equal(
      vaultAccount.owner.toString(),
      payer.publicKey.toString(),
      "Vault owner가 일치해야 함"
    );
    assert.equal(
      vaultAccount.vaultTokenAccount.toString(),
      vaultTokenAccount.toString(),
      "Vault Token Account가 일치해야 함"
    );

    console.log("Vault 초기화 성공");
  });

  it("Tom이 사용자 금고 계정을 초기화한다", async () => {
    const tx = await program.methods
      .initializeUserVault()
      .accounts({
        user: tom.publicKey,
        vault: vaultPda,
        userVaultAccount: tomUserVault,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([tom])
      .rpc();

    console.log("Tom UserVault 초기화 트랜잭션:", tx);

    const userVaultAccount = await program.account.userVaultAccount.fetch(
      tomUserVault
    );
    assert.equal(
      userVaultAccount.user.toString(),
      tom.publicKey.toString(),
      "User가 일치해야 함"
    );
    assert.equal(
      userVaultAccount.depositedAmount.toNumber(),
      0,
      "초기 잔액은 0이어야 함"
    );

    console.log("Tom UserVault 초기화 성공");
  });

  it("Dustin이 사용자 금고 계정을 초기화한다", async () => {
    const tx = await program.methods
      .initializeUserVault()
      .accounts({
        user: dustin.publicKey,
        vault: vaultPda,
        userVaultAccount: dustinUserVault,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([dustin])
      .rpc();

    console.log("Dustin UserVault 초기화 트랜잭션:", tx);

    const userVaultAccount = await program.account.userVaultAccount.fetch(
      dustinUserVault
    );
    assert.equal(
      userVaultAccount.depositedAmount.toNumber(),
      0,
      "초기 잔액은 0이어야 함"
    );

    console.log("Dustin UserVault 초기화 성공");
  });

  it("Tom이 300 토큰을 입금한다", async () => {
    const depositAmount = new BN(300 * 1_000_000); // 300 토큰

    const tx = await program.methods
      .deposit(depositAmount)
      .accounts({
        user: tom.publicKey,
        userTokenAccount: tomTokenAccount,
        vault: vaultPda,
        vaultTokenAccount: vaultTokenAccount,
        userVaultAccount: tomUserVault,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([tom])
      .rpc();

    console.log("Tom 입금 트랜잭션:", tx);

    // UserVault 잔액 확인
    const userVaultAccount = await program.account.userVaultAccount.fetch(
      tomUserVault
    );
    assert.equal(
      userVaultAccount.depositedAmount.toNumber(),
      300 * 1_000_000,
      "입금 후 잔액이 300이어야 함"
    );

    // Tom의 Token Account 잔액 확인
    const tomTokenAccountInfo = await getAccount(
      provider.connection,
      tomTokenAccount
    );
    assert.equal(
      Number(tomTokenAccountInfo.amount),
      700 * 1_000_000,
      "Tom의 토큰 잔액이 700이어야 함"
    );

    // Vault Token Account 잔액 확인
    const vaultTokenAccountInfo = await getAccount(
      provider.connection,
      vaultTokenAccount
    );
    assert.equal(
      Number(vaultTokenAccountInfo.amount),
      300 * 1_000_000,
      "Vault의 토큰 잔액이 300이어야 함"
    );

    console.log("Tom 입금 성공! 잔액: 300 토큰");
  });

  it("Dustin이 200 토큰을 입금한다", async () => {
    const depositAmount = new BN(200 * 1_000_000); // 200 토큰

    const tx = await program.methods
      .deposit(depositAmount)
      .accounts({
        user: dustin.publicKey,
        userTokenAccount: dustinTokenAccount,
        vault: vaultPda,
        vaultTokenAccount: vaultTokenAccount,
        userVaultAccount: dustinUserVault,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([dustin])
      .rpc();

    console.log("Dustin 입금 트랜잭션:", tx);

    // UserVault 잔액 확인
    const userVaultAccount = await program.account.userVaultAccount.fetch(
      dustinUserVault
    );
    assert.equal(
      userVaultAccount.depositedAmount.toNumber(),
      200 * 1_000_000,
      "입금 후 잔액이 200이어야 함"
    );

    // Vault Token Account 총 잔액 확인 (Tom 300 + Dustin 200 = 500)
    const vaultTokenAccountInfo = await getAccount(
      provider.connection,
      vaultTokenAccount
    );
    assert.equal(
      Number(vaultTokenAccountInfo.amount),
      500 * 1_000_000,
      "Vault의 총 토큰 잔액이 500이어야 함"
    );

    console.log("Dustin 입금 성공! Vault 총 잔액: 500 토큰");
  });

  it("Tom이 100 토큰을 출금한다", async () => {
    const withdrawAmount = new BN(100 * 1_000_000); // 100 토큰

    const tx = await program.methods
      .withdraw(withdrawAmount)
      .accounts({
        user: tom.publicKey,
        userTokenAccount: tomTokenAccount,
        vault: vaultPda,
        vaultTokenAccount: vaultTokenAccount,
        userVaultAccount: tomUserVault,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([tom])
      .rpc();

    console.log("Tom 출금 트랜잭션:", tx);

    // UserVault 잔액 확인 (300 - 100 = 200)
    const userVaultAccount = await program.account.userVaultAccount.fetch(
      tomUserVault
    );
    assert.equal(
      userVaultAccount.depositedAmount.toNumber(),
      200 * 1_000_000,
      "출금 후 잔액이 200이어야 함"
    );

    // Tom의 Token Account 잔액 확인 (700 + 100 = 800)
    const tomTokenAccountInfo = await getAccount(
      provider.connection,
      tomTokenAccount
    );
    assert.equal(
      Number(tomTokenAccountInfo.amount),
      800 * 1_000_000,
      "Tom의 토큰 잔액이 800이어야 함"
    );

    // Vault Token Account 잔액 확인 (500 - 100 = 400)
    const vaultTokenAccountInfo = await getAccount(
      provider.connection,
      vaultTokenAccount
    );
    assert.equal(
      Number(vaultTokenAccountInfo.amount),
      400 * 1_000_000,
      "Vault의 토큰 잔액이 400이어야 함"
    );

    console.log("Tom 출금 성공! 남은 잔액: 200 토큰");
  });

  it("Dustin이 전액(200 토큰)을 출금한다", async () => {
    const withdrawAmount = new BN(200 * 1_000_000); // 200 토큰

    const tx = await program.methods
      .withdraw(withdrawAmount)
      .accounts({
        user: dustin.publicKey,
        userTokenAccount: dustinTokenAccount,
        vault: vaultPda,
        vaultTokenAccount: vaultTokenAccount,
        userVaultAccount: dustinUserVault,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([dustin])
      .rpc();

    console.log("Dustin 전액 출금 트랜잭션:", tx);

    // UserVault 잔액 확인 (200 - 200 = 0)
    const userVaultAccount = await program.account.userVaultAccount.fetch(
      dustinUserVault
    );
    assert.equal(
      userVaultAccount.depositedAmount.toNumber(),
      0,
      "전액 출금 후 잔액이 0이어야 함"
    );

    // Dustin의 Token Account 잔액 확인 (300 + 200 = 500)
    const dustinTokenAccountInfo = await getAccount(
      provider.connection,
      dustinTokenAccount
    );
    assert.equal(
      Number(dustinTokenAccountInfo.amount),
      500 * 1_000_000,
      "Dustin의 토큰 잔액이 500이어야 함"
    );

    // Vault Token Account 잔액 확인 (400 - 200 = 200, Tom의 것만 남음)
    const vaultTokenAccountInfo = await getAccount(
      provider.connection,
      vaultTokenAccount
    );
    assert.equal(
      Number(vaultTokenAccountInfo.amount),
      200 * 1_000_000,
      "Vault의 토큰 잔액이 200이어야 함 (Tom의 잔액)"
    );

    console.log("Dustin 전액 출금 성공! Vault 남은 잔액: 200 토큰 (Tom의 것)");
  });

  it("잔액 이상으로 출금하려고 하면 실패한다", async () => {
    const withdrawAmount = new BN(300 * 1_000_000); // 300 토큰 (Tom 잔액은 200)

    try {
      await program.methods
        .withdraw(withdrawAmount)
        .accounts({
          user: tom.publicKey,
          userTokenAccount: tomTokenAccount,
          vault: vaultPda,
          vaultTokenAccount: vaultTokenAccount,
          userVaultAccount: tomUserVault,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        })
        .signers([tom])
        .rpc();

      assert.fail("잔액 부족 시 에러가 발생해야 함");
    } catch (error) {
      console.log("예상대로 에러 발생:", error.message);
      assert.include(
        error.message,
        "InsufficientBalance",
        "InsufficientBalance 에러가 발생해야 함"
      );
    }
  });
});
