use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Mint, TokenAccount, Transfer, Burn};

declare_id!("G3MS7ERZaPLH6ayr7u7fBwvPWtx1JCYksPEd4TEfxRzY");

#[program]
pub mod tournament_betting_system {
    use super::*;

    // 初始化合约
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.authority = ctx.accounts.authority.key();
        state.wanzi_mint = ctx.accounts.wanzi_mint.key();
        state.matchp_mint = ctx.accounts.matchp_mint.key();
        state.vote_mint = ctx.accounts.vote_mint.key();
        Ok(())
    }

    // 创建赛事
    pub fn create_tournament(
        ctx: Context<CreateTournament>,
        name: String,
        stake_amount: u64,
    ) -> Result<()> {
        let tournament = &mut ctx.accounts.tournament;
        tournament.authority = ctx.accounts.authority.key();
        tournament.name = name;
        tournament.stake_amount = stake_amount;
        tournament.is_active = true;
        tournament.is_staked = false;
        tournament.created_at = Clock::get()?.unix_timestamp;
        
        // 如果赛事方质押了matchp
        if stake_amount > 0 {
            // 转移matchp到合约账户
            let cpi_accounts = Transfer {
                from: ctx.accounts.authority_matchp_token.to_account_info(),
                to: ctx.accounts.tournament_matchp_token.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            token::transfer(cpi_ctx, stake_amount)?;
            
            tournament.is_staked = true;
        }
        
        Ok(())
    }

    // 下注
    pub fn place_bet(
        ctx: Context<PlaceBet>,
        amount: u64,
    ) -> Result<()> {
        let tournament = &mut ctx.accounts.tournament;
        require!(tournament.is_active, TournamentError::TournamentNotActive);
        
        let bet = &mut ctx.accounts.bet;
        bet.tournament = tournament.key();
        bet.team = ctx.accounts.team.key();
        bet.user = ctx.accounts.user.key();
        bet.amount = amount;
        bet.created_at = Clock::get()?.unix_timestamp;
        bet.is_settled = false;
        bet.is_winner = false;
        
        // 根据赛事是否质押决定使用哪种代币下注
        if tournament.is_staked {
            // 使用vote下注
            let cpi_accounts = Transfer {
                from: ctx.accounts.user_vote_token.to_account_info(),
                to: ctx.accounts.tournament_vote_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            token::transfer(cpi_ctx, amount)?;
        } else {
            // 使用wanzi下注
            let cpi_accounts = Transfer {
                from: ctx.accounts.user_wanzi_token.to_account_info(),
                to: ctx.accounts.tournament_wanzi_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            token::transfer(cpi_ctx, amount)?;
        }
        
        Ok(())
    }

    // 修改关闭赛事函数
    pub fn close_tournament(ctx: Context<CloseTournament>) -> Result<()> {
        // 先获取所有必要的值，避免同时可变和不可变借用
        let is_active = ctx.accounts.tournament.is_active;
        let is_staked = ctx.accounts.tournament.is_staked;
        let stake_amount = ctx.accounts.tournament.stake_amount;
        let bump = ctx.accounts.tournament.bump;
        let vote_token_amount = ctx.accounts.tournament_vote_token.amount;

        // 检查赛事是否激活
        require!(is_active, TournamentError::TournamentNotActive);
        
        // 更新赛事状态
        ctx.accounts.tournament.is_active = false;
        
        // 如果赛事方质押了matchp，返还matchp
        if is_staked {
            let seeds = &[
                b"tournament".as_ref(),
                &[bump],
            ];
            let signer = &[&seeds[..]];

            // 销毁所有vote代币
            let burn_accounts = Burn {
                mint: ctx.accounts.vote_mint.to_account_info(),
                from: ctx.accounts.tournament_vote_token.to_account_info(),
                authority: ctx.accounts.tournament.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                burn_accounts,
                signer,
            );
            token::burn(cpi_ctx, vote_token_amount)?;

            // 返还matchp
            let transfer_accounts = Transfer {
                from: ctx.accounts.tournament_matchp_token.to_account_info(),
                to: ctx.accounts.authority_matchp_token.to_account_info(),
                authority: ctx.accounts.tournament.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                transfer_accounts,
                signer,
            );
            token::transfer(cpi_ctx, stake_amount)?;
        }
        
        Ok(())
    }

    // 添加结算函数
    pub fn settle_bet(
        ctx: Context<SettleBet>,
        is_winner: bool,
    ) -> Result<()> {
        let bet = &mut ctx.accounts.bet;
        let tournament = &ctx.accounts.tournament;
        
        require!(!tournament.is_active, TournamentError::TournamentNotClosed);
        require!(!bet.is_settled, TournamentError::BetAlreadySettled);
        
        if tournament.is_staked {
            // 如果是vote下注，赢家获得vote
            if is_winner {
                let transfer_accounts = Transfer {
                    from: ctx.accounts.tournament_vote_token.to_account_info(),
                    to: ctx.accounts.user_vote_token.to_account_info(),
                    authority: ctx.accounts.tournament.to_account_info(),
                };
                let seeds = &[
                    b"tournament".as_ref(),
                    &[tournament.bump],
                ];
                let signer = &[&seeds[..]];
                let cpi_ctx = CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    transfer_accounts,
                    signer,
                );
                token::transfer(cpi_ctx, bet.amount)?;
            }
        } else {
            // 如果是wanzi下注，赢家获得wanzi
            if is_winner {
                let transfer_accounts = Transfer {
                    from: ctx.accounts.tournament_wanzi_token.to_account_info(),
                    to: ctx.accounts.user_wanzi_token.to_account_info(),
                    authority: ctx.accounts.tournament.to_account_info(),
                };
                let seeds = &[
                    b"tournament".as_ref(),
                    &[tournament.bump],
                ];
                let signer = &[&seeds[..]];
                let cpi_ctx = CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    transfer_accounts,
                    signer,
                );
                token::transfer(cpi_ctx, bet.amount)?;
            }
        }
        
        bet.is_settled = true;
        bet.is_winner = is_winner;
        
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        init,
        payer = authority,
        space = 8 + State::LEN
    )]
    pub state: Account<'info, State>,
    
    pub wanzi_mint: Box<Account<'info, Mint>>,
    pub matchp_mint: Box<Account<'info, Mint>>,
    pub vote_mint: Box<Account<'info, Mint>>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct CreateTournament<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        init,
        payer = authority,
        space = 8 + Tournament::LEN,
        seeds = [b"tournament", authority.key().as_ref()],
        bump
    )]
    pub tournament: Account<'info, Tournament>,
    
    #[account(mut)]
    pub authority_matchp_token: Box<Account<'info, TokenAccount>>,
    
    #[account(
        init,
        payer = authority,
        token::mint = matchp_mint,
        token::authority = tournament,
        seeds = [b"tournament_matchp", tournament.key().as_ref()],
        bump
    )]
    pub tournament_matchp_token: Box<Account<'info, TokenAccount>>,
    
    pub matchp_mint: Box<Account<'info, Mint>>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(mut)]
    pub tournament: Account<'info, Tournament>,
    
    #[account(
        init,
        payer = user,
        space = 8 + Bet::LEN
    )]
    pub bet: Account<'info, Bet>,
    
    pub team: Account<'info, Team>,
    
    #[account(mut)]
    pub user_wanzi_token: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub user_vote_token: Box<Account<'info, TokenAccount>>,
    
    #[account(mut)]
    pub tournament_wanzi_token: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub tournament_vote_token: Box<Account<'info, TokenAccount>>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct CloseTournament<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        mut,
        has_one = authority,
    )]
    pub tournament: Account<'info, Tournament>,
    
    #[account(mut)]
    pub tournament_matchp_token: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub authority_matchp_token: Box<Account<'info, TokenAccount>>,
    
    #[account(mut)]
    pub tournament_vote_token: Box<Account<'info, TokenAccount>>,
    
    pub vote_mint: Box<Account<'info, Mint>>,
    
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct State {
    pub authority: Pubkey,
    pub wanzi_mint: Pubkey,
    pub matchp_mint: Pubkey,
    pub vote_mint: Pubkey,
}

impl State {
    pub const LEN: usize = 32 + 32 + 32 + 32;
}

#[account]
pub struct Tournament {
    pub authority: Pubkey,
    pub name: String,
    pub stake_amount: u64,
    pub is_active: bool,
    pub is_staked: bool,
    pub created_at: i64,
    pub bump: u8,
}

impl Tournament {
    pub const LEN: usize = 32 + 32 + 8 + 1 + 1 + 8 + 1;
}

#[account]
pub struct Team {
    pub tournament: Pubkey,
    pub name: String,
    pub group: String,
}

impl Team {
    pub const LEN: usize = 32 + 32 + 32;
}

#[account]
pub struct Bet {
    pub tournament: Pubkey,
    pub team: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
    pub created_at: i64,
    pub is_settled: bool,
    pub is_winner: bool,
}

impl Bet {
    pub const LEN: usize = 32 + 32 + 32 + 8 + 8 + 1 + 1;
}

#[derive(Accounts)]
pub struct SettleBet<'info> {
    #[account(mut)]
    pub tournament: Account<'info, Tournament>,
    
    #[account(
        mut,
        constraint = bet.tournament == tournament.key()
    )]
    pub bet: Account<'info, Bet>,
    
    #[account(
        mut,
        constraint = user_wanzi_token.owner == bet.user,
        constraint = user_wanzi_token.mint == state.wanzi_mint
    )]
    pub user_wanzi_token: Box<Account<'info, TokenAccount>>,
    
    #[account(
        mut,
        constraint = user_vote_token.owner == bet.user,
        constraint = user_vote_token.mint == state.vote_mint
    )]
    pub user_vote_token: Box<Account<'info, TokenAccount>>,
    
    #[account(
        mut,
        constraint = tournament_wanzi_token.owner == tournament.key(),
        constraint = tournament_wanzi_token.mint == state.wanzi_mint
    )]
    pub tournament_wanzi_token: Box<Account<'info, TokenAccount>>,
    
    #[account(
        mut,
        constraint = tournament_vote_token.owner == tournament.key(),
        constraint = tournament_vote_token.mint == state.vote_mint
    )]
    pub tournament_vote_token: Box<Account<'info, TokenAccount>>,
    
    pub vote_mint: Box<Account<'info, Mint>>,
    
    pub state: Account<'info, State>,
    
    pub token_program: Program<'info, Token>,
}

#[error_code]
pub enum TournamentError {
    #[msg("Tournament is not active")]
    TournamentNotActive,
    #[msg("Tournament is not closed")]
    TournamentNotClosed,
    #[msg("Bet is already settled")]
    BetAlreadySettled,
    #[msg("Invalid token account")]
    InvalidTokenAccount,
    #[msg("Invalid tournament authority")]
    InvalidTournamentAuthority,
}