const anchor = require('@coral-xyz/anchor');
const { WANZI_MINT, MATCHP_MINT, VOTE_MINT } = require('./token-addresses');
const { Program } = require('@coral-xyz/anchor');
const { PublicKey } = require('@solana/web3.js');
const idl = require('./target/idl/tournament_betting_system.json');

// 配置连接
const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);

// 程序ID
const programId = new PublicKey('FL7hr4LCxSTk2p2xwbk2Qy6dnGeBxc6Daz223dzfTncz');
const program = new Program(idl, programId, provider);

async function main() {
  console.log("初始化合约...");
  
  // 创建状态账户
  const statePda = await PublicKey.findProgramAddressSync(
    [
      Buffer.from("state"),
      provider.wallet.publicKey.toBuffer()
    ],
    program.programId
  )[0];
  
  try {
    // 初始化合约
    const tx = await program.methods
      .initialize()
      .accounts({
        authority: provider.wallet.publicKey,
        state: statePda,
        wanziMint: new PublicKey(WANZI_MINT),
        matchpMint: new PublicKey(MATCHP_MINT),
        voteMint: new PublicKey(VOTE_MINT),
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .rpc();
    
    console.log("合约初始化成功! 交易ID:", tx);
    console.log("状态账户地址:", statePda.toString());
  } catch (err) {
    console.error("初始化失败:", err);
  }
}

main().then(
  () => process.exit(),
  err => {
    console.error(err);
    process.exit(-1);
  }
); 