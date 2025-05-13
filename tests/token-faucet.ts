import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, createMint, createAccount } from '@solana/spl-token';
import { assert } from "chai";

// 导入代币合约的IDL
import idl from "../target/idl/token_faucet.json";

describe("token_faucet", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // 加载代币合约的程序
  const programId = new PublicKey("TokenFaucetProgram1111111111111111111111111");
  const program = new anchor.Program(idl, programId, provider);
  
  let wanziMint: PublicKey;
  let matchpMint: PublicKey;
  let voteMint: PublicKey;
  let faucetPda: PublicKey;
  
  // 管理员钱包
  const authorityWallet = provider.wallet;
  
  // 管理员代币账户
  let authorityWanziAccount: PublicKey;
  let authorityMatchpAccount: PublicKey;
  let authorityVoteAccount: PublicKey;
  
  // 测试用户钱包
  const userWallet = anchor.web3.Keypair.generate();
  
  // 用户代币账户
  let userWanziAccount: PublicKey;
  let userMatchpAccount: PublicKey;
  let userVoteAccount: PublicKey;
  
  it("准备测试环境", async () => {
    // 给测试用户转一些SOL
    const signature = await provider.connection.requestAirdrop(
      userWallet.publicKey,
      1000000000
    );
    await provider.connection.confirmTransaction(signature);
    
    // 创建代币
    wanziMint = await createMint(
      provider.connection,
      authorityWallet.payer,
      authorityWallet.publicKey,
      null,
      9
    );
    
    matchpMint = await createMint(
      provider.connection,
      authorityWallet.payer,
      authorityWallet.publicKey,
      null,
      9
    );
    
    voteMint = await createMint(
      provider.connection,
      authorityWallet.payer,
      authorityWallet.publicKey,
      null,
      9
    );
    
    // 创建管理员代币账户
    authorityWanziAccount = await createAccount(
      provider.connection,
      authorityWallet.payer,
      wanziMint,
      authorityWallet.publicKey
    );
    
    authorityMatchpAccount = await createAccount(
      provider.connection,
      authorityWallet.payer,
      matchpMint,
      authorityWallet.publicKey
    );
    
    authorityVoteAccount = await createAccount(
      provider.connection,
      authorityWallet.payer,
      voteMint,
      authorityWallet.publicKey
    );
    
    // 创建用户代币账户
    userWanziAccount = await createAccount(
      provider.connection,
      authorityWallet.payer,
      wanziMint,
      userWallet.publicKey
    );
    
    userMatchpAccount = await createAccount(
      provider.connection,
      authorityWallet.payer,
      matchpMint,
      userWallet.publicKey
    );
    
    userVoteAccount = await createAccount(
      provider.connection,
      authorityWallet.payer,
      voteMint,
      userWallet.publicKey
    );
    
    // 初始化faucet PDA
    faucetPda = PublicKey.findProgramAddressSync(
      [Buffer.from("faucet")],
      program.programId
    )[0];
  });

  it("初始化代币铸造器", async () => {
    try {
      await program.methods
        .initialize()
        .accounts({
          authority: authorityWallet.publicKey,
          faucet: faucetPda,
          wanziMint,
          matchpMint,
          voteMint,
          authorityWanziToken: authorityWanziAccount,
          authorityMatchpToken: authorityMatchpAccount,
          authorityVoteToken: authorityVoteAccount,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .rpc();
        
      const faucet = await program.account.faucet.fetch(faucetPda);
      assert.ok(faucet.authority.equals(authorityWallet.publicKey));
      assert.ok(faucet.wanziMint.equals(wanziMint));
      assert.ok(faucet.matchpMint.equals(matchpMint));
      assert.ok(faucet.voteMint.equals(voteMint));
    } catch (error) {
      console.log("初始化错误:", error);
      throw error;
    }
  });

  it("铸造代币", async () => {
    try {
      await program.methods
        .mintTokens()
        .accounts({
          authority: authorityWallet.publicKey,
          faucet: faucetPda,
          wanziMint,
          matchpMint,
          voteMint,
          authorityWanziToken: authorityWanziAccount,
          authorityMatchpToken: authorityMatchpAccount,
          authorityVoteToken: authorityVoteAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
        
      // 检查管理员账户余额
      const authorityWanziBalance = await provider.connection.getTokenAccountBalance(authorityWanziAccount);
      const authorityMatchpBalance = await provider.connection.getTokenAccountBalance(authorityMatchpAccount);
      const authorityVoteBalance = await provider.connection.getTokenAccountBalance(authorityVoteAccount);
      
      console.log("铸造后余额：", {
        wanzi: authorityWanziBalance.value.amount,
        matchp: authorityMatchpBalance.value.amount,
        vote: authorityVoteBalance.value.amount,
      });
      
      assert.equal(authorityWanziBalance.value.amount, "500000000000000000");
      assert.equal(authorityMatchpBalance.value.amount, "500000000000000000");
      assert.equal(authorityVoteBalance.value.amount, "500000000000000000");
    } catch (error) {
      console.log("铸造错误:", error);
      throw error;
    }
  });

  it("用户领取代币", async () => {
    try {
      await program.methods
        .claimTokens()
        .accounts({
          authority: authorityWallet.publicKey,
          faucet: faucetPda,
          user: userWallet.publicKey,
          userWanziToken: userWanziAccount,
          userMatchpToken: userMatchpAccount,
          userVoteToken: userVoteAccount,
          authorityWanziToken: authorityWanziAccount,
          authorityMatchpToken: authorityMatchpAccount,
          authorityVoteToken: authorityVoteAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
        
      // 检查用户账户余额
      const userWanziBalance = await provider.connection.getTokenAccountBalance(userWanziAccount);
      const userMatchpBalance = await provider.connection.getTokenAccountBalance(userMatchpAccount);
      const userVoteBalance = await provider.connection.getTokenAccountBalance(userVoteAccount);
      
      console.log("用户领取后余额：", {
        wanzi: userWanziBalance.value.amount,
        matchp: userMatchpBalance.value.amount,
        vote: userVoteBalance.value.amount,
      });
      
      assert.equal(userWanziBalance.value.amount, "100000000000");
      assert.equal(userMatchpBalance.value.amount, "100000000000");
      assert.equal(userVoteBalance.value.amount, "100000000000");
      
      // 检查用户是否被记录为已领取
      const faucet = await program.account.faucet.fetch(faucetPda);
      assert.ok(faucet.claimedUsers.some(key => key.equals(userWallet.publicKey)));
    } catch (error) {
      console.log("领取错误:", error);
      throw error;
    }
  });

  it("用户不能重复领取代币", async () => {
    try {
      await program.methods
        .claimTokens()
        .accounts({
          authority: authorityWallet.publicKey,
          faucet: faucetPda,
          user: userWallet.publicKey,
          userWanziToken: userWanziAccount,
          userMatchpToken: userMatchpAccount,
          userVoteToken: userVoteAccount,
          authorityWanziToken: authorityWanziAccount,
          authorityMatchpToken: authorityMatchpAccount,
          authorityVoteToken: authorityVoteAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
      assert.fail("应该抛出错误");
    } catch (error) {
      console.log("预期的错误:", error.message);
      assert.include(error.message, "User has already claimed tokens");
    }
  });
}); 