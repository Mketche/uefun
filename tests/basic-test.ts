import { expect } from 'chai';

describe('基础测试', () => {
  it('可以运行基本断言', () => {
    // 一些基本的断言
    expect(1 + 1).to.equal(2);
    expect('hello').to.be.a('string');
    expect({ key: 'value' }).to.have.property('key');
    
    console.log('基本断言测试通过！');
  });
}); 