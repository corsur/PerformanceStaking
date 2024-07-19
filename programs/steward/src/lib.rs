#![allow(clippy::redundant_pub_crate)]
use anchor_lang::prelude::*;
use anchor_lang::IdlBuild;
use instructions::*;

use crate::utils::PreferredValidatorType;

mod allocator;
pub mod constants;
pub mod delegation;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod score;
pub mod state;
pub mod utils;

pub use state::*;

declare_id!("sssh4zkKhX8jXTNQz1xDHyGpygzgu2UhcRcUvZihBjP");

/*
This program manages the selection of validators and delegation of stake for a SPL Stake Pool.

It relies on validator metrics collected by the Validator History Program.

To initialize a Steward-managed pool:
1) `initialize_config` - creates Config account, and transfers ownership of the pool's staker authority to the Staker PDA
2) `initialize_state` - creates State account
3) `realloc_state` - increases the size of the State account to StewardStateAccount::SIZE, and initializes values once at that size

Each cycle, the following steps are performed by a permissionless cranker:
1) compute_score (once per validator)
2) compute_delegations
3) idle
4) compute_instant_unstake (once per validator)
5) rebalance (once per validator)

For the remaining epochs in a cycle, the state will repeat idle->compute_instant_unstake->rebalance.
After `num_epochs_between_scoring` epochs, the state can transition back to ComputeScores.


If manual intervention is required, the following spl-stake-pool instructions are available, and can be executed by the config.authority:
- `add_validator_to_pool`
- `remove_validator_from_pool`
- `set_preferred_validator`
- `increase_validator_stake`
- `decrease_validator_stake`
- `increase_additional_validator_stake`
- `decrease_additional_validator_stake`
- `redelegate`
- `set_staker`
*/
#[program]
pub mod steward {
    use super::*;

    /* Initialization instructions */

    // Initializes Config and Staker accounts. Must be called before any other instruction
    // Requires Pool to be initialized
    pub fn initialize_steward(
        ctx: Context<InitializeSteward>,
        update_parameters_args: UpdateParametersArgs,
    ) -> Result<()> {
        instructions::initialize_steward::handler(ctx, &update_parameters_args)
    }

    /// Increases state account by 10KiB each ix until it reaches StewardStateAccount::SIZE
    pub fn realloc_state(ctx: Context<ReallocState>) -> Result<()> {
        instructions::realloc_state::handler(ctx)
    }

    /* Main cycle loop */

    /// Adds a validator to the pool if it has a validator history account, matches stake_minimum, and is not yet in the pool
    pub fn auto_add_validator_to_pool(ctx: Context<AutoAddValidator>) -> Result<()> {
        instructions::auto_add_validator_to_pool::handler(ctx)
    }

    /// Removes a validator from the pool if its stake account is inactive or the vote account has closed
    pub fn auto_remove_validator_from_pool(
        ctx: Context<AutoRemoveValidator>,
        validator_list_index: u64,
    ) -> Result<()> {
        instructions::auto_remove_validator_from_pool::handler(ctx, validator_list_index as usize)
    }

    /// Interrupts when the validator list length does not match the expected length
    pub fn index_mismatch_interrupt(
        ctx: Context<IndexMismatchInterrupt>,
        validator_index_to_remove: u64,
    ) -> Result<()> {
        instructions::index_mismatch_interrupt::handler(ctx, validator_index_to_remove as usize)
    }

    /// Housekeeping, run at the start of any new epoch before any other instructions
    pub fn epoch_maintenance(ctx: Context<EpochMaintenance>) -> Result<()> {
        instructions::epoch_maintenance::handler(ctx)
    }

    /// Computes score for a the validator at `validator_list_index` for the current cycle.
    pub fn compute_score(ctx: Context<ComputeScore>, validator_list_index: u64) -> Result<()> {
        instructions::compute_score::handler(ctx, validator_list_index as usize)
    }

    /// Computes delegation for a validator for the current cycle.
    /// All validators must have delegations computed before stake can be delegated
    pub fn compute_delegations(ctx: Context<ComputeDelegations>) -> Result<()> {
        instructions::compute_delegations::handler(ctx)
    }

    /// Idle state, waiting for epoch progress before transitioning to next state
    pub fn idle(ctx: Context<Idle>) -> Result<()> {
        instructions::idle::handler(ctx)
    }

    /// Checks if a validator at `validator_list_index` should be instant unstaked, and marks it if so
    pub fn compute_instant_unstake(
        ctx: Context<ComputeInstantUnstake>,
        validator_list_index: u64,
    ) -> Result<()> {
        instructions::compute_instant_unstake::handler(ctx, validator_list_index as usize)
    }

    /// Increases or decreases stake for a validator at `validator_list_index` to match the target stake,
    /// given constraints on increase/decrease priority, reserve balance, and unstaking caps
    pub fn rebalance(ctx: Context<Rebalance>, validator_list_index: u64) -> Result<()> {
        instructions::rebalance::handler(ctx, validator_list_index as usize)
    }

    /* Admin instructions */

    // If `new_authority` is not a pubkey you own, you cannot regain the authority, but you can
    // use the stake pool manager to set a new staker
    pub fn set_new_authority(
        ctx: Context<SetNewAuthority>,
        authority_type: AuthorityType,
    ) -> Result<()> {
        instructions::set_new_authority::handler(ctx, authority_type)
    }

    pub fn pause_steward(ctx: Context<PauseSteward>) -> Result<()> {
        instructions::pause_steward::handler(ctx)
    }

    pub fn resume_steward(ctx: Context<ResumeSteward>) -> Result<()> {
        instructions::resume_steward::handler(ctx)
    }

    /// Adds the validator at `index` to the blacklist. It will be instant unstaked and never receive delegations
    pub fn add_validator_to_blacklist(
        ctx: Context<AddValidatorToBlacklist>,
        validator_history_blacklist: u32,
    ) -> Result<()> {
        instructions::add_validator_to_blacklist::handler(ctx, validator_history_blacklist)
    }

    /// Removes the validator at `index` from the blacklist
    pub fn remove_validator_from_blacklist(
        ctx: Context<RemoveValidatorFromBlacklist>,
        validator_history_blacklist: u32,
    ) -> Result<()> {
        instructions::remove_validator_from_blacklist::handler(ctx, validator_history_blacklist)
    }

    /// For parameters that are present in args, the instruction checks that they are within sensible bounds and saves them to config struct
    pub fn update_parameters(
        ctx: Context<UpdateParameters>,
        update_parameters_args: UpdateParametersArgs,
    ) -> Result<()> {
        instructions::update_parameters::handler(ctx, &update_parameters_args)
    }

    /* TEMPORARY ADMIN INSTRUCTIONS for testing */

    /// Resets steward state account to its initial state.
    pub fn reset_steward_state(ctx: Context<ResetStewardState>) -> Result<()> {
        instructions::reset_steward_state::handler(ctx)
    }

    /// Closes Steward PDA accounts associated with a given Config (StewardStateAccount, and Staker).
    /// Config is not closed as it is a Keypair, so lamports can simply be withdrawn.
    /// Reclaims lamports to authority
    pub fn close_steward_accounts(ctx: Context<CloseStewardAccounts>) -> Result<()> {
        instructions::close_steward_accounts::handler(ctx)
    }

    /* Passthrough instructions to spl-stake-pool, where the signer is Staker. Must be invoked by `config.authority` */

    pub fn set_staker(ctx: Context<SetStaker>) -> Result<()> {
        instructions::spl_passthrough::set_staker_handler(ctx)
    }

    pub fn add_validator_to_pool(
        ctx: Context<AddValidatorToPool>,
        validator_seed: Option<u32>,
    ) -> Result<()> {
        instructions::spl_passthrough::add_validator_to_pool_handler(ctx, validator_seed)
    }

    pub fn remove_validator_from_pool(
        ctx: Context<RemoveValidatorFromPool>,
        validator_list_index: u64,
    ) -> Result<()> {
        instructions::spl_passthrough::remove_validator_from_pool_handler(
            ctx,
            validator_list_index as usize,
        )
    }

    pub fn set_preferred_validator(
        ctx: Context<SetPreferredValidator>,
        validator_type: PreferredValidatorType,
        validator: Option<Pubkey>,
    ) -> Result<()> {
        instructions::spl_passthrough::set_preferred_validator_handler(
            ctx,
            validator_type.as_ref(),
            validator,
        )
    }

    pub fn increase_validator_stake(
        ctx: Context<IncreaseValidatorStake>,
        lamports: u64,
        transient_seed: u64,
    ) -> Result<()> {
        instructions::spl_passthrough::increase_validator_stake_handler(
            ctx,
            lamports,
            transient_seed,
        )
    }

    pub fn decrease_validator_stake(
        ctx: Context<DecreaseValidatorStake>,
        lamports: u64,
        transient_seed: u64,
    ) -> Result<()> {
        instructions::spl_passthrough::decrease_validator_stake_handler(
            ctx,
            lamports,
            transient_seed,
        )
    }

    pub fn increase_additional_validator_stake(
        ctx: Context<IncreaseAdditionalValidatorStake>,
        lamports: u64,
        transient_seed: u64,
        ephemeral_seed: u64,
    ) -> Result<()> {
        instructions::spl_passthrough::increase_additional_validator_stake_handler(
            ctx,
            lamports,
            transient_seed,
            ephemeral_seed,
        )
    }

    pub fn decrease_additional_validator_stake(
        ctx: Context<DecreaseAdditionalValidatorStake>,
        lamports: u64,
        transient_seed: u64,
        ephemeral_seed: u64,
    ) -> Result<()> {
        instructions::spl_passthrough::decrease_additional_validator_stake_handler(
            ctx,
            lamports,
            transient_seed,
            ephemeral_seed,
        )
    }
}
