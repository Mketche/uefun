import { expect } from 'chai';
import * as fs from 'fs';

describe('合约编译测试', () => {
  it('token_faucet应该已经成功编译', () => {
    const programPath = 'target/deploy/token_faucet.so';
    expect(fs.existsSync(programPath)).to.be.true;
    console.log(`找到编译后的程序: ${programPath}`);
  });
}); 