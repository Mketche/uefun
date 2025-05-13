use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, MintTo};

declare_id!("TokenFaucetProgram1111111111111111111111111");

#[program]
pub mod token_faucet {
    use super::*;

    /// 初始化代币水龙头合约
    /// 设置管理员权限和相关代币铸造器
    pub fn initialize(
        ctx: Context<Initialize>,
    ) -> Result<()> {
        let faucet = &mut ctx.accounts.faucet;
        // 设置合约管理员
        faucet.authority = ctx.accounts.authority.key();
        // 记录wanzi代币铸造器地址
        faucet.wanzi_mint = ctx.accounts.wanzi_mint.key();
        // 记录matchp代币铸造器地址
        faucet.matchp_mint = ctx.accounts.matchp_mint.key();
        // 初始化已领取用户列表为空
        faucet.claimed_users = Vec::new();
        Ok(())
    }

    /// 铸造代币函数
    /// 由管理员调用，向管理员账户铸造wanzi和matchp代币
    pub fn mint_tokens(
        ctx: Context<MintTokens>,
    ) -> Result<()> {
        // 铸造5亿枚代币，考虑9位小数
        let amount = 500 * 10u64.pow(9);

        // 铸造wanzi代币到管理员账户
        let cpi_accounts = MintTo {
            mint: ctx.accounts.wanzi_mint.to_account_info(),
            to: ctx.accounts.authority_wanzi_token.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program.clone(), cpi_accounts);
        token::mint_to(cpi_ctx, amount)?;

        // 铸造matchp代币到管理员账户
        let cpi_accounts = MintTo {
            mint: ctx.accounts.matchp_mint.to_account_info(),
            to: ctx.accounts.authority_matchp_token.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::mint_to(cpi_ctx, amount)?;

        Ok(())
    }

    /// 用户领取代币函数
    /// 每个用户只能领取一次，每次可获得100枚wanzi和matchp代币
    pub fn claim_tokens(
        ctx: Context<ClaimTokens>,
    ) -> Result<()> {
        // 检查用户是否已经领取过代币
        let faucet = &mut ctx.accounts.faucet;
        require!(
            !faucet.claimed_users.contains(&ctx.accounts.user.key()),
            FaucetError::AlreadyClaimed
        );

        // 每次领取100枚代币，考虑9位小数
        let amount = 100 * 10u64.pow(9);

        // 从管理员账户转移wanzi代币到用户账户
        let cpi_accounts = token::Transfer {
            from: ctx.accounts.authority_wanzi_token.to_account_info(),
            to: ctx.accounts.user_wanzi_token.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program.clone(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        // 从管理员账户转移matchp代币到用户账户
        let cpi_accounts = token::Transfer {
            from: ctx.accounts.authority_matchp_token.to_account_info(),
            to: ctx.accounts.user_matchp_token.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        // 记录用户已领取，防止重复领取
        faucet.claimed_users.push(ctx.accounts.user.key());

        Ok(())
    }
}

/// 初始化指令所需的账户结构
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// 合约管理员，支付初始化费用
    #[account(mut)]
    pub authority: Signer<'info>,

    /// 水龙头账户，使用PDA地址
    #[account(
        init,
        payer = authority,
        space = 8 + Faucet::LEN,
        seeds = [b"faucet"],
        bump
    )]
    pub faucet: Account<'info, Faucet>,

    /// wanzi代币铸造器
    pub wanzi_mint: Box<Account<'info, Mint>>,
    /// matchp代币铸造器
    pub matchp_mint: Box<Account<'info, Mint>>,

    /// 管理员的wanzi代币账户
    #[account(mut)]
    pub authority_wanzi_token: Box<Account<'info, TokenAccount>>,
    /// 管理员的matchp代币账户
    #[account(mut)]
    pub authority_matchp_token: Box<Account<'info, TokenAccount>>,

    /// 系统程序
    pub system_program: Program<'info, System>,
    /// 代币程序
    pub token_program: Program<'info, Token>,
    /// 租金系统变量
    pub rent: Sysvar<'info, Rent>,
}

/// 铸造代币指令所需的账户结构
#[derive(Accounts)]
pub struct MintTokens<'info> {
    /// 合约管理员，必须是签名者
    #[account(mut)]
    pub authority: Signer<'info>,

    /// 水龙头账户，验证调用者是管理员
    #[account(
        mut,
        has_one = authority,
    )]
    pub faucet: Account<'info, Faucet>,

    /// wanzi代币铸造器
    pub wanzi_mint: Box<Account<'info, Mint>>,
    /// matchp代币铸造器
    pub matchp_mint: Box<Account<'info, Mint>>,

    /// 管理员的wanzi代币账户
    #[account(mut)]
    pub authority_wanzi_token: Box<Account<'info, TokenAccount>>,
    /// 管理员的matchp代币账户
    #[account(mut)]
    pub authority_matchp_token: Box<Account<'info, TokenAccount>>,

    /// 代币程序
    pub token_program: Program<'info, Token>,
}

/// 领取代币指令所需的账户结构
#[derive(Accounts)]
pub struct ClaimTokens<'info> {
    /// 合约管理员，必须是签名者
    #[account(mut)]
    pub authority: Signer<'info>,

    /// 水龙头账户，验证调用者是管理员
    #[account(
        mut,
        has_one = authority,
    )]
    pub faucet: Account<'info, Faucet>,

    /// 领取代币的用户账户
    /// CHECK: 这是要领取代币的用户
    pub user: AccountInfo<'info>,

    /// 用户的wanzi代币账户
    #[account(mut)]
    pub user_wanzi_token: Box<Account<'info, TokenAccount>>,
    /// 用户的matchp代币账户
    #[account(mut)]
    pub user_matchp_token: Box<Account<'info, TokenAccount>>,

    /// 管理员的wanzi代币账户
    #[account(mut)]
    pub authority_wanzi_token: Box<Account<'info, TokenAccount>>,
    /// 管理员的matchp代币账户
    #[account(mut)]
    pub authority_matchp_token: Box<Account<'info, TokenAccount>>,

    /// 代币程序
    pub token_program: Program<'info, Token>,
}

/// 水龙头合约数据结构
#[account]
pub struct Faucet {
    /// 管理员公钥
    pub authority: Pubkey,
    /// wanzi代币铸造器地址
    pub wanzi_mint: Pubkey,
    /// matchp代币铸造器地址
    pub matchp_mint: Pubkey,
    /// 已领取代币的用户列表
    pub claimed_users: Vec<Pubkey>,
}

impl Faucet {
    /// 水龙头账户数据大小，预留1000个用户空间
    pub const LEN: usize = 32 + 32 + 32 + 4 + (32 * 1000); // 预留1000个用户的空间
}

/// 错误码定义
#[error_code]
pub enum FaucetError {
    /// 用户已经领取过代币，不能重复领取
    #[msg("User has already claimed tokens")]
    AlreadyClaimed,
} 