use anchor_lang::prelude::*;
use solana_program::pubkey::Pubkey;

mod state;
mod errors;
mod utils;

declare_id!("YOUR_PROGRAM_ID_HERE"); // Replace with your actual deployed program ID

#[program]
pub mod five90 {
    use super::*;

    /// Initialize the program state
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let state = &mut ctx.accounts.five90_state;
        state.admin = ctx.accounts.admin.key();
        state.epoch = 0;
        Ok(())
    }

    /// Update stake allocations based on validator performance
    pub fn update_stake_allocations(ctx: Context<UpdateStakeAllocations>, epoch: u64) -> Result<()> {
        let state = &mut ctx.accounts.five90_state;
        require!(epoch >= state.epoch, errors::Five90Error::InvalidEpoch);

        let mut validator_scores = vec![];

        // Fetch Validator History data
        for validator in ctx.remaining_accounts.iter() {
            if let Some(stats) = utils::fetch_validator_history(&validator.key(), epoch) {
                let staking_rewards = stats.epoch_credits - (stats.epoch_credits * stats.commission as u64 / 100);
                validator_scores.push((stats.vote_account, staking_rewards));
            }
        }

        // Sort validators by staking rewards (higher is better)
        validator_scores.sort_by(|a, b| b.1.cmp(&a.1));

        // Select top 90th percentile validators
        let cutoff_index = validator_scores.len() * 9 / 10;
        let top_validators = &validator_scores[..cutoff_index];

        // Undelegate stake from underperforming validators
        for validator in ctx.remaining_accounts.iter() {
            if !top_validators.iter().any(|(vote_account, _)| *vote_account == validator.key()) {
                msg!("Undelegating stake from validator: {}", validator.key());
                utils::undelegate_stake(&validator.key())?;
            }
        }

        // Delegate stake to top 90th percentile validators
        for (vote_account, _) in top_validators {
            msg!("Delegating stake to validator: {}", vote_account);
            utils::delegate_stake(vote_account)?;
        }

        state.epoch = epoch;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = admin, space = 8 + 40)]
    pub five90_state: Account<'info, Five90State>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateStakeAllocations<'info> {
    #[account(mut, has_one = admin)]
    pub five90_state: Account<'info, Five90State>,
    pub admin: Signer<'info>,
}