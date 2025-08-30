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
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: This is the new collection being created.
    #[account(mut)]
    pub collection: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 8 + 2 + 2 + 32,
        seeds = [b"state", collection.key().as_ref()],
        bump
    )]
    pub state: Account<'info, CollectionState>,

    /// CHECK: The update authority is just an address that will be assigned authority in the CPI.
    pub update_authority: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: Validated against the Metaplex Core program ID in the instruction.
    pub mpl_core_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct MintLockedAsset<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: This is the new asset being created.
    #[account(mut)]
    pub asset: Signer<'info>,

    #[account(
        seeds = [b"state", collection.key().as_ref()],
        bump,
        has_one = collection,
        has_one = update_authority
    )]
    pub state: Account<'info, CollectionState>,

    /// CHECK: The collection account is validated by the `state` PDA.
    pub collection: UncheckedAccount<'info>,

    pub update_authority: Signer<'info>,

    /// CHECK: The intended owner of the new asset.
    pub owner: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: Validated against the Metaplex Core program ID in the instruction.
    pub mpl_core_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct UnlockAsset<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: This is the asset being modified by the CPI.
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: The collection account is needed to derive and validate the state PDA.
    pub collection: UncheckedAccount<'info>,

    #[account(
        seeds = [b"state", collection.key().as_ref()],
        bump,
        has_one = update_authority,
        has_one = collection
    )]
    pub state: Account<'info, CollectionState>,

    pub update_authority: Signer<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: Validated against the Metaplex Core program ID in the instruction.
    pub mpl_core_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct AddPlugin<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: This is the asset being modified by the CPI.
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: The collection account is needed to derive and validate the state PDA.
    pub collection: UncheckedAccount<'info>,

    #[account(
        seeds = [b"state", collection.key().as_ref()],
        bump,
        has_one = update_authority,
        has_one = collection
    )]
    pub state: Account<'info, CollectionState>,

    pub update_authority: Signer<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: Validated against the Metaplex Core program ID in the instruction.
    pub mpl_core_program: UncheckedAccount<'info>,
}
