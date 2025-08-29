#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use mpl_core::instructions::{
    AddPluginV1CpiBuilder, CreateCollectionV2CpiBuilder, CreateV2CpiBuilder,
    UpdatePluginV1CpiBuilder,
};
use mpl_core::types::{
    Creator, FreezeDelegate, Plugin, PluginAuthority, PluginAuthorityPair, Royalties, RuleSet,
};

declare_id!("8gvyrTcF8GmbWuHPwqhChZg2iQ3FvtxC1vzhRYuyhL9J");

#[program]
pub mod earnnft {
    use super::*;

    /// Create a Core Collection and store time-lock and royalty tiers in PDA state.
    pub fn create_collection_with_state(
        ctx: Context<CreateCollectionWithState>,
        name: String,
        uri: String,
        unlock_ts: i64,
        pre_unlock_bps: u16,
        post_unlock_bps: u16,
    ) -> Result<()> {
        require!(pre_unlock_bps <= 10000, CustomError::InvalidBps);
        require!(post_unlock_bps <= 10000, CustomError::InvalidBps);
        require!(unlock_ts > 0, CustomError::InvalidUnlockTs);

        // Create the collection via Core CPI without plugins.
        let mut b = CreateCollectionV2CpiBuilder::new(&ctx.accounts.mpl_core_program);
        b.collection(&ctx.accounts.collection)
            .payer(&ctx.accounts.payer)
            .update_authority(Some(&ctx.accounts.update_authority))
            .system_program(&ctx.accounts.system_program)
            .name(name)
            .uri(uri)
            .plugins(vec![]);
        b.invoke()?;

        // Store policy/state in the PDA
        let st = &mut ctx.accounts.state;
        st.collection = ctx.accounts.collection.key();
        st.unlock_ts = unlock_ts;
        st.pre_unlock_bps = pre_unlock_bps;
        st.post_unlock_bps = post_unlock_bps;
        st.update_authority = ctx.accounts.update_authority.key();

        Ok(())
    }

    /// Mint an asset into the collection with asset-level royalties set to pre_unlock_bps
    /// and a FreezeDelegate plugin that starts frozen.
    pub fn mint_locked_asset(
        ctx: Context<MintLockedAsset>,
        name: String,
        uri: String,
    ) -> Result<()> {
        // Prepare plugins for the asset
        let royalties = PluginAuthorityPair {
            plugin: Plugin::Royalties(Royalties {
                basis_points: ctx.accounts.state.pre_unlock_bps,
                creators: vec![Creator {
                    address: ctx.accounts.update_authority.key(),
                    percentage: 100, // 100% to update authority for demo
                }],
                rule_set: RuleSet::None,
            }),
            authority: Some(PluginAuthority::UpdateAuthority),
        };

        // FreezeDelegate plugin: initially frozen; will be thawed on unlock.
        let freeze = PluginAuthorityPair {
            plugin: Plugin::FreezeDelegate(FreezeDelegate { frozen: true }),
            authority: Some(PluginAuthority::UpdateAuthority),
        };

        let mut b = CreateV2CpiBuilder::new(&ctx.accounts.mpl_core_program);
        b.asset(&ctx.accounts.asset)
            .collection(Some(&ctx.accounts.collection))
            .payer(&ctx.accounts.payer)
            .system_program(&ctx.accounts.system_program)
            .update_authority(Some(&ctx.accounts.update_authority))
            .owner(Some(&ctx.accounts.owner)) // provide asset owner
            .name(name)
            .uri(uri)
            .plugins(vec![royalties, freeze]);
        b.invoke()?;

        Ok(())
    }

    /// Unlock the asset after unlock_ts:
    /// - Thaw FreezeDelegate (frozen = false)
    /// - Update Royalties to post_unlock_bps
    pub fn unlock_asset(ctx: Context<UnlockAsset>) -> Result<()> {
        let clock = Clock::get()?;
        require!(
            clock.unix_timestamp >= ctx.accounts.state.unlock_ts,
            CustomError::NotYetUnlocked
        );

        // 1) Thaw FreezeDelegate (set frozen = false)
        let mut up = UpdatePluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program);
        up.asset(&ctx.accounts.asset)
            .collection(None)
            .payer(&ctx.accounts.payer)
            .system_program(&ctx.accounts.system_program)
            .authority(Some(&ctx.accounts.update_authority))
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }));
        up.invoke()?;

        // 2) Update Royalties to the configured post-unlock basis points
        let mut update_royalties = UpdatePluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program);

        update_royalties
            .asset(&ctx.accounts.asset)
            .collection(None)
            .payer(&ctx.accounts.payer)
            .system_program(&ctx.accounts.system_program)
            .authority(Some(&ctx.accounts.update_authority))
            .plugin(Plugin::Royalties(Royalties {
                basis_points: ctx.accounts.state.post_unlock_bps,
                creators: vec![Creator {
                    address: ctx.accounts.update_authority.key(),
                    percentage: 100,
                }],
                rule_set: RuleSet::None,
            }));
        update_royalties.invoke()?;

        Ok(())
    }

    /// Add or replace a plugin on an asset (for example, royalties).
    pub fn add_plugin(ctx: Context<AddPlugin>, plugin: Plugin) -> Result<()> {
        let mut ap = AddPluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program);
        ap.asset(&ctx.accounts.asset)
            .collection(None)
            .payer(&ctx.accounts.payer)
            .system_program(&ctx.accounts.system_program)
            .authority(Some(&ctx.accounts.update_authority))
            .plugin(plugin);
        ap.invoke()?;
        Ok(())
    }
}

/* ----------------------------- Accounts & State ---------------------------- */

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
    /// New Core collection account (signer when not a PDA)
    #[account(mut)]
    pub collection: Signer<'info>,

    /// Payer for rent and transaction fees
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Update authority that controls collection and plugin changes
    pub update_authority: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 8 + 2 + 2 + 32,
        seeds = [b"state", collection.key().as_ref()],
        bump
    )]
    pub state: Account<'info, CollectionState>,

    /// Metaplex Core program id (unchecked)
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintLockedAsset<'info> {
    /// New Core asset account (signer when not a PDA)
    #[account(mut)]
    pub asset: Signer<'info>,

    /// Existing collection account (owned by Core; passed to CPI only)
    pub collection: UncheckedAccount<'info>,

    /// Owner of the new asset (intended owner)
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

    /// Metaplex Core program id (unchecked)
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UnlockAsset<'info> {
    /// Core asset account (unchecked)
    pub asset: UncheckedAccount<'info>,

    /// Collection account bound to the state (unchecked)
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

    /// Metaplex Core program id (unchecked)
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddPlugin<'info> {
    /// Asset account (unchecked)
    pub asset: UncheckedAccount<'info>,
    /// Collection account (unchecked)
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

    /// Metaplex Core program id (unchecked)
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/* --------------------------------- Errors --------------------------------- */

#[error_code]
pub enum CustomError {
    #[msg("Invalid basis points")]
    InvalidBps,
    #[msg("Invalid unlock timestamp")]
    InvalidUnlockTs,
    #[msg("Current time is before unlock")]
    NotYetUnlocked,
    #[msg("State does not match collection")]
    StateCollectionMismatch,
    #[msg("Update authority mismatch")]
    BadAuthority,
}
