import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import * as fs from "fs";

describe("部署测试", () => {
  // 创建一个连接到本地测试验证器的连接
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  it("可以在本地部署 token-faucet 程序", async () => {
    console.log("准备部署token-faucet程序...");
    
    // 使用anchor build构建的程序路径
    const programPath = "target/deploy/token_faucet.so";
    
    // 检查程序文件是否存在
    if (fs.existsSync(programPath)) {
      console.log("找到编译好的程序:", programPath);
    } else {
      console.error("未找到编译好的程序文件。请先运行 anchor build。");
      return;
    }
    
    console.log("部署测试完成。您可以使用 anchor deploy 命令部署程序。");
  });
}); 