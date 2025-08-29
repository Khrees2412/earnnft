use crate::errors::CustomError;
use anchor_lang::prelude::*;

#[account]
pub struct CollectionState {
    pub collection: Pubkey,
    pub unlock_ts: i64,
    pub pre_unlock_bps: u16,
    pub post_unlock_bps: u16,
    pub update_authority: Pubkey,
}

#[derive(Accounts)]
pub struct CreateCollectionWithState<'info> {
    // New collection account (must sign if not a PDA)
    #[account(mut)]
    pub collection: Signer<'info>,

    // Payer for rent/compute
    #[account(mut)]
    pub payer: Signer<'info>,

    // Update authority for collection and plugins
    pub update_authority: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 8 + 2 + 2 + 32,
        seeds = [b"state", collection.key().as_ref()],
        bump
    )]
    pub state: Account<'info, CollectionState>,

    // Metaplex Core program (validated at runtime)
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintLockedAsset<'info> {
    // New core asset (signer when not PDA)
    #[account(mut)]
    pub asset: Signer<'info>,

    // Existing collection (passed to CPI)
    pub collection: UncheckedAccount<'info>,

    // Intended owner of the minted asset
    pub owner: UncheckedAccount<'info>,

    #[account(
        seeds = [b"state", collection.key().as_ref()],
        bump,
        constraint = state.collection == collection.key() @ CustomError::StateCollectionMismatch,
        constraint = state.update_authority == update_authority.key() @ CustomError::BadAuthority
    )]
    pub state: Account<'info, CollectionState>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub update_authority: Signer<'info>,

    // Metaplex Core program (validated at runtime)
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UnlockAsset<'info> {
    // Core asset updated by CPI; must be writable
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,

    // Collection bound to state
    pub collection: UncheckedAccount<'info>,

    #[account(
        seeds = [b"state", collection.key().as_ref()],
        bump,
        constraint = state.collection == collection.key() @ CustomError::StateCollectionMismatch,
        constraint = state.update_authority == update_authority.key() @ CustomError::BadAuthority
    )]
    pub state: Account<'info, CollectionState>,

    #[account(mut)]
    pub payer: Signer<'info>,
    pub update_authority: Signer<'info>,

    // Metaplex Core program (validated at runtime)
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddPlugin<'info> {
    // Asset that will receive the plugin; must be writable for CPI
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,
    pub collection: UncheckedAccount<'info>,

    #[account(
        seeds = [b"state", collection.key().as_ref()],
        bump,
        constraint = state.collection == collection.key() @ CustomError::StateCollectionMismatch,
        constraint = state.update_authority == update_authority.key() @ CustomError::BadAuthority
    )]
    pub state: Account<'info, CollectionState>,

    #[account(mut)]
    pub payer: Signer<'info>,
    pub update_authority: Signer<'info>,

    // Metaplex Core program (validated at runtime)
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}
