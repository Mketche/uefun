use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Mint, TokenAccount, Transfer, Burn, MintTo};

declare_id!("FL7hr4LCxSTk2p2xwbk2Qy6dnGeBxc6Daz223dzfTncz");

#[program]
pub mod tournament_betting_system {
    use super::*;

    /// 初始化下注系统合约
    /// 设置管理员权限和相关代币铸造器地址
    pub fn initialize(ctx: Context<Initialize>, token_faucet_program_id: Pubkey) -> Result<()> {
        let state = &mut ctx.accounts.state;
        // 设置合约管理员
        state.authority = ctx.accounts.authority.key();
        // 记录wanzi代币铸造器地址
        state.wanzi_mint = ctx.accounts.wanzi_mint.key();
        // 记录matchp代币铸造器地址
        state.matchp_mint = ctx.accounts.matchp_mint.key();
        // 记录vote代币铸造器地址
        state.vote_mint = ctx.accounts.vote_mint.key();
        // 记录代币水龙头程序ID
        state.token_faucet_program_id = token_faucet_program_id;
        Ok(())
    }

    /// 创建赛事
    /// 设置赛事名称和质押金额，如果质押matchp则获得等量vote代币
    pub fn create_tournament(
        ctx: Context<CreateTournament>,
        name: String,
        stake_amount: u64,
    ) -> Result<()> {
        let tournament = &mut ctx.accounts.tournament;
        // 设置赛事管理员
        tournament.authority = ctx.accounts.authority.key();
        // 设置赛事名称
        tournament.name = name;
        // 设置质押金额
        tournament.stake_amount = stake_amount;
        // 设置赛事为激活状态
        tournament.is_active = true;
        // 初始化为未质押状态
        tournament.is_staked = false;
        // 记录创建时间
        tournament.created_at = Clock::get()?.unix_timestamp;
        // 记录PDA bump
        tournament.bump = ctx.bumps.tournament;
        
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
            
            // 铸造vote给赛事方
            let mint_accounts = MintTo {
                mint: ctx.accounts.vote_mint.to_account_info(),
                to: ctx.accounts.authority_vote_token.to_account_info(),
                authority: ctx.accounts.vote_mint_authority.to_account_info(),
            };
            let cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                mint_accounts,
            );
            token::mint_to(cpi_ctx, stake_amount)?;
            
            // 更新赛事为已质押状态
            tournament.is_staked = true;
        }
        
        Ok(())
    }

    /// 创建比赛轮次/组别
    /// 设置轮次名称和序号，用于组织比赛流程
    pub fn create_tournament_round(
        ctx: Context<CreateTournamentRound>,
        name: String,
        round_number: u8,
    ) -> Result<()> {
        let round = &mut ctx.accounts.round;
        // 关联到特定赛事
        round.tournament = ctx.accounts.tournament.key();
        // 设置轮次名称
        round.name = name;
        // 设置轮次序号
        round.round_number = round_number;
        // 设置轮次为激活状态
        round.is_active = true;
        // 初始化为未完成状态
        round.is_completed = false;
        // 记录创建时间
        round.created_at = Clock::get()?.unix_timestamp;
        // 记录PDA bump
        round.bump = ctx.bumps.round;
        Ok(())
    }

    /// 创建团队
    /// 设置团队名称，关联到特定赛事和轮次
    pub fn create_team(
        ctx: Context<CreateTeam>,
        name: String,
    ) -> Result<()> {
        let team = &mut ctx.accounts.team;
        // 关联到特定赛事
        team.tournament = ctx.accounts.tournament.key();
        // 关联到特定轮次
        team.round = ctx.accounts.round.key();
        // 设置团队名称
        team.name = name;
        // 初始化为非获胜状态
        team.is_winner = false;
        // 初始化为未淘汰状态
        team.is_eliminated = false;
        // 记录PDA bump
        team.bump = ctx.bumps.team;
        Ok(())
    }

    /// 下注
    /// 用户对特定轮次的团队进行下注，根据赛事是否质押决定使用wanzi或vote代币
    pub fn place_bet(
        ctx: Context<PlaceBet>,
        amount: u64,
    ) -> Result<()> {
        let tournament = &ctx.accounts.tournament;
        let round = &ctx.accounts.round;
        
        // 检查赛事是否激活
        require!(tournament.is_active, TournamentError::TournamentNotActive);
        // 检查轮次是否激活
        require!(round.is_active, TournamentError::RoundNotActive);
        // 检查团队是否已被淘汰
        require!(!ctx.accounts.team.is_eliminated, TournamentError::TeamEliminated);
        
        let bet = &mut ctx.accounts.bet;
        // 关联到特定赛事
        bet.tournament = tournament.key();
        // 关联到特定轮次
        bet.round = round.key();
        // 关联到特定团队
        bet.team = ctx.accounts.team.key();
        // 记录下注用户
        bet.user = ctx.accounts.user.key();
        // 记录下注金额
        bet.amount = amount;
        // 记录下注时间
        bet.created_at = Clock::get()?.unix_timestamp;
        // 初始化为未结算状态
        bet.is_settled = false;
        // 初始化为非获胜状态
        bet.is_winner = false;
        
        // 根据赛事是否质押决定使用哪种代币下注
        if tournament.is_staked {
            // 检查用户是否有足够的vote代币
            require!(
                ctx.accounts.user_vote_token.amount >= amount,
                TournamentError::InsufficientTokenBalance
            );
            
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
            // 检查用户是否有足够的wanzi代币
            require!(
                ctx.accounts.user_wanzi_token.amount >= amount,
                TournamentError::InsufficientTokenBalance
            );
            
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

    /// 完成轮次
    /// 标记获胜团队，更新轮次状态
    pub fn complete_round(
        ctx: Context<CompleteRound>,
        winner_team_pubkey: Pubkey,
    ) -> Result<()> {
        let round = &mut ctx.accounts.round;
        // 检查轮次是否激活
        require!(round.is_active, TournamentError::RoundNotActive);
        
        // 检查传入的获胜队伍是否属于这个轮次
        require!(
            ctx.accounts.winner_team.round == round.key(),
            TournamentError::TeamNotInRound
        );
        
        // 确保传入的获胜队伍公钥与实际账户一致
        require!(
            ctx.accounts.winner_team.key() == winner_team_pubkey,
            TournamentError::InvalidTeam
        );
        
        // 标记轮次为完成状态
        round.is_active = false;
        round.is_completed = true;
        
        // 更新获胜队伍状态
        let winner_team = &mut ctx.accounts.winner_team;
        winner_team.is_winner = true;
        
        Ok(())
    }

    /// 结算下注
    /// 根据轮次结果结算用户下注，获胜者获得奖励
    pub fn settle_bet(
        ctx: Context<SettleBet>,
    ) -> Result<()> {
        let bet = &mut ctx.accounts.bet;
        let round = &ctx.accounts.round;
        let winner_team = &ctx.accounts.winner_team;
        
        // 检查轮次是否已完成
        require!(round.is_completed, TournamentError::RoundNotCompleted);
        // 检查下注是否已结算
        require!(!bet.is_settled, TournamentError::BetAlreadySettled);
        // 检查下注是否属于该轮次
        require!(bet.round == round.key(), TournamentError::BetNotInRound);
        
        // 检查下注的队伍是否是获胜队伍
        let is_winner = bet.team == winner_team.key() && winner_team.is_winner;
        
        // 如果是赢家，转移奖励
        if is_winner {
            if ctx.accounts.tournament.is_staked {
                // 使用vote奖励
                let seeds = &[
                    b"tournament".as_ref(),
                    ctx.accounts.tournament.authority.as_ref(),
                    &[ctx.accounts.tournament.bump],
                ];
                let signer = &[&seeds[..]];
                
                let transfer_accounts = Transfer {
                    from: ctx.accounts.tournament_vote_token.to_account_info(),
                    to: ctx.accounts.user_vote_token.to_account_info(),
                    authority: ctx.accounts.tournament.to_account_info(),
                };
                let cpi_ctx = CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    transfer_accounts,
                    signer,
                );
                token::transfer(cpi_ctx, bet.amount)?;
            } else {
                // 使用wanzi奖励
                let seeds = &[
                    b"tournament".as_ref(),
                    ctx.accounts.tournament.authority.as_ref(),
                    &[ctx.accounts.tournament.bump],
                ];
                let signer = &[&seeds[..]];
                
                let transfer_accounts = Transfer {
                    from: ctx.accounts.tournament_wanzi_token.to_account_info(),
                    to: ctx.accounts.user_wanzi_token.to_account_info(),
                    authority: ctx.accounts.tournament.to_account_info(),
                };
                let cpi_ctx = CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    transfer_accounts,
                    signer,
                );
                token::transfer(cpi_ctx, bet.amount)?;
            }
        }
        
        // 更新下注状态
        bet.is_settled = true;
        bet.is_winner = is_winner;
        
        Ok(())
    }

    /// 关闭赛事
    /// 结束赛事，处理质押的代币和奖励
    pub fn close_tournament(ctx: Context<CloseTournament>) -> Result<()> {
        // 先获取所有必要的值，避免同时可变和不可变借用
        let is_active = ctx.accounts.tournament.is_active;
        let is_staked = ctx.accounts.tournament.is_staked;
        let stake_amount = ctx.accounts.tournament.stake_amount;
        let bump = ctx.accounts.tournament.bump;
        let authority_key = ctx.accounts.authority.key();

        // 检查赛事是否激活
        require!(is_active, TournamentError::TournamentNotActive);
        
        // 更新赛事状态为关闭
        ctx.accounts.tournament.is_active = false;
        
        // 如果赛事方质押了matchp
        if is_staked {
            let authority_seeds = authority_key.as_ref();
            let seeds = &[
                b"tournament".as_ref(),
                authority_seeds,
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
            token::burn(cpi_ctx, ctx.accounts.tournament_vote_token.amount)?;

            // 返还matchp到管理员账户
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
        } else {
            // 如果使用wanzi下注，返还到管理员地址
            let authority_seeds = authority_key.as_ref();
            let seeds = &[
                b"tournament".as_ref(),
                authority_seeds,
                &[bump],
            ];
            let signer = &[&seeds[..]];

            // 返还wanzi到管理员地址
            let transfer_accounts = Transfer {
                from: ctx.accounts.tournament_wanzi_token.to_account_info(),
                to: ctx.accounts.authority_wanzi_token.to_account_info(),
                authority: ctx.accounts.tournament.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                transfer_accounts,
                signer,
            );
            token::transfer(cpi_ctx, ctx.accounts.tournament_wanzi_token.amount)?;
        }
        
        Ok(())
    }
    
    /// 更新代币水龙头程序地址
    /// 允许管理员更新代币水龙头程序ID
    pub fn update_token_faucet_address(
        ctx: Context<UpdateTokenFaucet>,
        new_token_faucet_program_id: Pubkey,
    ) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.token_faucet_program_id = new_token_faucet_program_id;
        Ok(())
    }
}

/// 初始化指令所需的账户结构
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// 合约管理员，支付初始化费用
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// 状态账户，使用PDA地址
    #[account(
        init,
        payer = authority,
        space = 8 + State::LEN,
        seeds = [b"state"],
        bump
    )]
    pub state: Account<'info, State>,
    
    /// wanzi代币铸造器
    pub wanzi_mint: Box<Account<'info, Mint>>,
    /// matchp代币铸造器
    pub matchp_mint: Box<Account<'info, Mint>>,
    /// vote代币铸造器
    pub vote_mint: Box<Account<'info, Mint>>,
    
    /// 系统程序
    pub system_program: Program<'info, System>,
    /// 代币程序
    pub token_program: Program<'info, Token>,
    /// 租金系统变量
    pub rent: Sysvar<'info, Rent>,
}

/// 创建赛事指令所需的账户结构
#[derive(Accounts)]
pub struct CreateTournament<'info> {
    /// 赛事管理员，支付创建费用
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// 赛事账户，使用PDA地址
    #[account(
        init,
        payer = authority,
        space = 8 + Tournament::LEN,
        seeds = [b"tournament", authority.key().as_ref()],
        bump
    )]
    pub tournament: Account<'info, Tournament>,
    
    /// 管理员的matchp代币账户
    #[account(mut)]
    pub authority_matchp_token: Box<Account<'info, TokenAccount>>,
    
    /// 管理员的vote代币账户
    #[account(mut)]
    pub authority_vote_token: Box<Account<'info, TokenAccount>>,
    
    /// 赛事的matchp代币账户
    #[account(
        init,
        payer = authority,
        token::mint = matchp_mint,
        token::authority = tournament,
        seeds = [b"tournament_matchp", tournament.key().as_ref()],
        bump
    )]
    pub tournament_matchp_token: Box<Account<'info, TokenAccount>>,
    
    /// 赛事的vote代币账户
    #[account(
        init,
        payer = authority,
        token::mint = vote_mint,
        token::authority = tournament,
        seeds = [b"tournament_vote", tournament.key().as_ref()],
        bump
    )]
    pub tournament_vote_token: Box<Account<'info, TokenAccount>>,
    
    /// matchp代币铸造器
    pub matchp_mint: Box<Account<'info, Mint>>,
    /// vote代币铸造器
    pub vote_mint: Box<Account<'info, Mint>>,
    /// vote代币铸造权限账户
    /// CHECK: 这是可以铸造vote代币的权限账户
    pub vote_mint_authority: AccountInfo<'info>,
    
    /// 系统程序
    pub system_program: Program<'info, System>,
    /// 代币程序
    pub token_program: Program<'info, Token>,
    /// 租金系统变量
    pub rent: Sysvar<'info, Rent>,
}

/// 创建轮次指令所需的账户结构
#[derive(Accounts)]
#[instruction(name: String, round_number: u8)]
pub struct CreateTournamentRound<'info> {
    /// 赛事管理员，支付创建费用
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// 赛事账户，验证调用者是管理员
    #[account(
        mut,
        constraint = tournament.authority == authority.key(),
        constraint = tournament.is_active == true
    )]
    pub tournament: Account<'info, Tournament>,
    
    /// 轮次账户，使用PDA地址
    #[account(
        init,
        payer = authority,
        space = 8 + TournamentRound::LEN,
        seeds = [b"round", tournament.key().as_ref(), &[round_number]],
        bump
    )]
    pub round: Account<'info, TournamentRound>,
    
    /// 系统程序
    pub system_program: Program<'info, System>,
    /// 租金系统变量
    pub rent: Sysvar<'info, Rent>,
}

/// 创建团队指令所需的账户结构
#[derive(Accounts)]
#[instruction(name: String)]
pub struct CreateTeam<'info> {
    /// 赛事管理员，支付创建费用
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// 赛事账户，验证调用者是管理员
    #[account(
        mut,
        constraint = tournament.authority == authority.key(),
        constraint = tournament.is_active == true
    )]
    pub tournament: Account<'info, Tournament>,
    
    /// 轮次账户，验证轮次属于该赛事
    #[account(
        constraint = round.tournament == tournament.key(),
        constraint = round.is_active == true
    )]
    pub round: Account<'info, TournamentRound>,
    
    /// 团队账户，使用PDA地址
    #[account(
        init,
        payer = authority,
        space = 8 + Team::LEN,
        seeds = [b"team", round.key().as_ref(), name.as_bytes()],
        bump
    )]
    pub team: Account<'info, Team>,
    
    /// 系统程序
    pub system_program: Program<'info, System>,
    /// 租金系统变量
    pub rent: Sysvar<'info, Rent>,
}

/// 下注指令所需的账户结构
#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct PlaceBet<'info> {
    /// 下注用户，支付下注费用
    #[account(mut)]
    pub user: Signer<'info>,
    
    /// 赛事账户，验证赛事是否激活
    #[account(
        constraint = tournament.is_active == true
    )]
    pub tournament: Account<'info, Tournament>,
    
    /// 轮次账户，验证轮次是否激活
    #[account(
        constraint = round.tournament == tournament.key(),
        constraint = round.is_active == true
    )]
    pub round: Account<'info, TournamentRound>,
    
    /// 团队账户，验证团队是否属于该轮次
    #[account(
        constraint = team.tournament == tournament.key(),
        constraint = team.round == round.key()
    )]
    pub team: Account<'info, Team>,
    
    /// 下注账户
    #[account(
        init,
        payer = user,
        space = 8 + Bet::LEN
    )]
    pub bet: Account<'info, Bet>,
    
    /// 用户的wanzi代币账户
    #[account(mut)]
    pub user_wanzi_token: Box<Account<'info, TokenAccount>>,
    /// 用户的vote代币账户
    #[account(mut)]
    pub user_vote_token: Box<Account<'info, TokenAccount>>,
    
    /// 赛事的wanzi代币账户
    #[account(
        init_if_needed,
        payer = user,
        token::mint = wanzi_mint,
        token::authority = tournament,
        seeds = [b"tournament_wanzi", tournament.key().as_ref()],
        bump
    )]
    pub tournament_wanzi_token: Box<Account<'info, TokenAccount>>,
    
    /// 赛事的vote代币账户
    #[account(
        init_if_needed,
        payer = user,
        token::mint = vote_mint,
        token::authority = tournament,
        seeds = [b"tournament_vote", tournament.key().as_ref()],
        bump
    )]
    pub tournament_vote_token: Box<Account<'info, TokenAccount>>,
    
    /// wanzi代币铸造器
    pub wanzi_mint: Box<Account<'info, Mint>>,
    /// vote代币铸造器
    pub vote_mint: Box<Account<'info, Mint>>,
    
    /// 系统程序
    pub system_program: Program<'info, System>,
    /// 代币程序
    pub token_program: Program<'info, Token>,
    /// 租金系统变量
    pub rent: Sysvar<'info, Rent>,
}

/// 完成轮次指令所需的账户结构
#[derive(Accounts)]
pub struct CompleteRound<'info> {
    /// 赛事管理员，必须是签名者
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// 赛事账户，验证调用者是管理员
    #[account(
        mut,
        constraint = tournament.authority == authority.key(),
        constraint = tournament.is_active == true
    )]
    pub tournament: Account<'info, Tournament>,
    
    /// 轮次账户，验证轮次是否激活
    #[account(
        mut,
        constraint = round.tournament == tournament.key(),
        constraint = round.is_active == true
    )]
    pub round: Account<'info, TournamentRound>,
    
    /// 获胜团队账户，验证团队是否属于该轮次
    #[account(
        mut,
        constraint = winner_team.tournament == tournament.key(),
        constraint = winner_team.round == round.key()
    )]
    pub winner_team: Account<'info, Team>,
}

/// 结算下注指令所需的账户结构
#[derive(Accounts)]
pub struct SettleBet<'info> {
    /// 赛事管理员，必须是签名者
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// 赛事账户
    pub tournament: Account<'info, Tournament>,
    
    /// 轮次账户，验证轮次是否已完成
    #[account(
        constraint = round.tournament == tournament.key(),
        constraint = round.is_completed == true
    )]
    pub round: Account<'info, TournamentRound>,
    
    /// 获胜团队账户，验证团队是否获胜
    #[account(
        constraint = winner_team.tournament == tournament.key(),
        constraint = winner_team.round == round.key(),
        constraint = winner_team.is_winner == true
    )]
    pub winner_team: Account<'info, Team>,
    
    /// 下注账户，验证下注是否未结算
    #[account(
        mut,
        constraint = bet.tournament == tournament.key(),
        constraint = bet.round == round.key(),
        constraint = bet.is_settled == false
    )]
    pub bet: Account<'info, Bet>,
    
    /// 下注用户账户
    /// CHECK: 这是下注的用户
    pub user: AccountInfo<'info>,
    
    /// 用户的wanzi代币账户
    #[account(
        mut,
        constraint = user_wanzi_token.owner == user.key()
    )]
    pub user_wanzi_token: Box<Account<'info, TokenAccount>>,
    
    /// 用户的vote代币账户
    #[account(
        mut,
        constraint = user_vote_token.owner == user.key()
    )]
    pub user_vote_token: Box<Account<'info, TokenAccount>>,
    
    /// 赛事的wanzi代币账户
    #[account(mut)]
    pub tournament_wanzi_token: Box<Account<'info, TokenAccount>>,
    /// 赛事的vote代币账户
    #[account(mut)]
    pub tournament_vote_token: Box<Account<'info, TokenAccount>>,
    
    /// 代币程序
    pub token_program: Program<'info, Token>,
}

/// 关闭赛事指令所需的账户结构
#[derive(Accounts)]
pub struct CloseTournament<'info> {
    /// 赛事管理员，必须是签名者
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// 赛事账户，验证调用者是管理员
    #[account(
        mut,
        has_one = authority,
    )]
    pub tournament: Account<'info, Tournament>,
    
    /// 赛事的matchp代币账户
    #[account(mut)]
    pub tournament_matchp_token: Box<Account<'info, TokenAccount>>,
    /// 管理员的matchp代币账户
    #[account(mut)]
    pub authority_matchp_token: Box<Account<'info, TokenAccount>>,
    
    /// 赛事的vote代币账户
    #[account(mut)]
    pub tournament_vote_token: Box<Account<'info, TokenAccount>>,
    /// 赛事的wanzi代币账户
    #[account(mut)]
    pub tournament_wanzi_token: Box<Account<'info, TokenAccount>>,
    /// 管理员的wanzi代币账户
    #[account(mut)]
    pub authority_wanzi_token: Box<Account<'info, TokenAccount>>,
    
    /// vote代币铸造器
    pub vote_mint: Box<Account<'info, Mint>>,
    
    /// 代币程序
    pub token_program: Program<'info, Token>,
}

/// 更新代币水龙头地址指令所需的账户结构
#[derive(Accounts)]
pub struct UpdateTokenFaucet<'info> {
    /// 合约管理员，必须是签名者
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// 状态账户，验证调用者是管理员
    #[account(
        mut,
        has_one = authority,
        seeds = [b"state"],
        bump
    )]
    pub state: Account<'info, State>,
}

/// 状态账户数据结构
#[account]
pub struct State {
    /// 管理员公钥
    pub authority: Pubkey,
    /// wanzi代币铸造器地址
    pub wanzi_mint: Pubkey,
    /// matchp代币铸造器地址
    pub matchp_mint: Pubkey,
    /// vote代币铸造器地址
    pub vote_mint: Pubkey,
    /// 代币水龙头程序ID
    pub token_faucet_program_id: Pubkey,
}

impl State {
    /// 状态账户数据大小
    pub const LEN: usize = 32 + 32 + 32 + 32 + 32;
}

/// 赛事账户数据结构
#[account]
pub struct Tournament {
    /// 管理员公钥
    pub authority: Pubkey,
    /// 赛事名称
    pub name: String,
    /// 质押金额
    pub stake_amount: u64,
    /// 是否激活
    pub is_active: bool,
    /// 是否已质押
    pub is_staked: bool,
    /// 创建时间
    pub created_at: i64,
    /// PDA bump
    pub bump: u8,
}

impl Tournament {
    /// 赛事账户数据大小
    pub const LEN: usize = 8 +  // discriminator
                          32 + // authority
                          32 + // name (max length)
                          8 +  // stake_amount
                          1 +  // is_active
                          1 +  // is_staked
                          8 +  // created_at
                          1;   // bump
}

/// 轮次账户数据结构
#[account]
pub struct TournamentRound {
    /// 关联的赛事
    pub tournament: Pubkey,
    /// 轮次名称，如"小组赛"、"半决赛"、"决赛"等
    pub name: String,
    /// 轮次序号
    pub round_number: u8,
    /// 轮次是否激活
    pub is_active: bool,
    /// 轮次是否已完成
    pub is_completed: bool,
    /// 创建时间
    pub created_at: i64,
    /// PDA bump
    pub bump: u8,
}

impl TournamentRound {
    /// 轮次账户数据大小
    pub const LEN: usize = 32 + // tournament
                          32 + // name (max length)
                          1 +  // round_number
                          1 +  // is_active
                          1 +  // is_completed
                          8 +  // created_at
                          1;   // bump
}

/// 团队账户数据结构
#[account]
pub struct Team {
    /// 关联的赛事
    pub tournament: Pubkey,
    /// 关联的轮次
    pub round: Pubkey,
    /// 队伍名称
    pub name: String,
    /// 是否为该轮次的获胜者
    pub is_winner: bool,
    /// 是否被淘汰
    pub is_eliminated: bool,
    /// PDA bump
    pub bump: u8,
}

impl Team {
    /// 团队账户数据大小
    pub const LEN: usize = 32 + // tournament
                          32 + // round
                          32 + // name (max length)
                          1 +  // is_winner
                          1 +  // is_eliminated
                          1;   // bump
}

/// 下注账户数据结构
#[account]
pub struct Bet {
    /// 关联的赛事
    pub tournament: Pubkey,
    /// 关联的轮次
    pub round: Pubkey,
    /// 下注的队伍
    pub team: Pubkey,
    /// 下注的用户
    pub user: Pubkey,
    /// 下注金额
    pub amount: u64,
    /// 创建时间
    pub created_at: i64,
    /// 是否已结算
    pub is_settled: bool,
    /// 是否获胜
    pub is_winner: bool,
}

impl Bet {
    /// 下注账户数据大小
    pub const LEN: usize = 32 + // tournament
                          32 + // round
                          32 + // team
                          32 + // user
                          8 +  // amount
                          8 +  // created_at
                          1 +  // is_settled
                          1;   // is_winner
}

/// 错误码定义
#[error_code]
pub enum TournamentError {
    /// 赛事未激活
    #[msg("Tournament is not active")]
    TournamentNotActive,
    /// 轮次未激活
    #[msg("Round is not active")]
    RoundNotActive,
    /// 轮次未完成
    #[msg("Round is not completed")]
    RoundNotCompleted,
    /// 下注不属于该轮次
    #[msg("Bet is not in this round")]
    BetNotInRound,
    /// 赛事未关闭
    #[msg("Tournament is not closed")]
    TournamentNotClosed,
    /// 下注已结算
    #[msg("Bet is already settled")]
    BetAlreadySettled,
    /// 无效的代币账户
    #[msg("Invalid token account")]
    InvalidTokenAccount,
    /// 无效的赛事管理员
    #[msg("Invalid tournament authority")]
    InvalidTournamentAuthority,
    /// 代币余额不足
    #[msg("Insufficient token balance")]
    InsufficientTokenBalance,
    /// 团队已被淘汰
    #[msg("Team is already eliminated")]
    TeamEliminated,
    /// 团队不属于该轮次
    #[msg("Team is not in this round")]
    TeamNotInRound,
    /// 无效的团队
    #[msg("Invalid team")]
    InvalidTeam,
}