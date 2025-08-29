#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use mpl_core::types::{
    Creator, FreezeDelegate, Plugin, PluginAuthority, PluginAuthorityPair, Royalties, RuleSet,
};
use mpl_core::{
    instructions::{
        AddPluginV1CpiBuilder, CreateCollectionV2CpiBuilder, CreateV2CpiBuilder,
        UpdatePluginV1CpiBuilder,
    },
    ID as MPL_CORE_ID,
};

mod state;
use state::state::*;

pub mod errors;
use errors::*;

declare_id!("8gvyrTcF8GmbWuHPwqhChZg2iQ3FvtxC1vzhRYuyhL9J");

#[program]
pub mod earnnft {

    use super::*;

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

        // Ensure the provided program id is the expected Metaplex Core program.
        require_keys_eq!(
            ctx.accounts.mpl_core_program.key(),
            MPL_CORE_ID,
            CustomError::InvalidMplCoreProgram
        );

        // Create the collection via CPI with no collection-level plugins.
        let mut b = CreateCollectionV2CpiBuilder::new(&ctx.accounts.mpl_core_program);
        b.collection(&ctx.accounts.collection)
            .payer(&ctx.accounts.payer)
            .update_authority(Some(&ctx.accounts.update_authority))
            .system_program(&ctx.accounts.system_program)
            .name(name)
            .uri(uri)
            .plugins(vec![]);
        b.invoke()?;

        // Persist state in PDA
        let st = &mut ctx.accounts.state;
        st.collection = ctx.accounts.collection.key();
        st.unlock_ts = unlock_ts;
        st.pre_unlock_bps = pre_unlock_bps;
        st.post_unlock_bps = post_unlock_bps;
        st.update_authority = ctx.accounts.update_authority.key();

        Ok(())
    }

    pub fn mint_locked_asset(
        ctx: Context<MintLockedAsset>,
        name: String,
        uri: String,
    ) -> Result<()> {
        // Validate core program id
        require_keys_eq!(
            ctx.accounts.mpl_core_program.key(),
            MPL_CORE_ID,
            CustomError::InvalidMplCoreProgram
        );

        // Build asset-level plugins: royalties (pre-unlock) and freeze (starts frozen)
        let royalties = PluginAuthorityPair {
            plugin: Plugin::Royalties(Royalties {
                basis_points: ctx.accounts.state.pre_unlock_bps,
                creators: vec![Creator {
                    address: ctx.accounts.update_authority.key(),
                    percentage: 100,
                }],
                rule_set: RuleSet::None,
            }),
            authority: Some(PluginAuthority::UpdateAuthority),
        };

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
            .owner(Some(&ctx.accounts.owner))
            .name(name)
            .uri(uri)
            .plugins(vec![royalties, freeze]);
        b.invoke()?;

        Ok(())
    }

    pub fn unlock_asset(ctx: Context<UnlockAsset>) -> Result<()> {
        let clock = Clock::get()?;
        require!(
            clock.unix_timestamp >= ctx.accounts.state.unlock_ts,
            CustomError::NotYetUnlocked
        );

        // Validate core program id
        require_keys_eq!(
            ctx.accounts.mpl_core_program.key(),
            MPL_CORE_ID,
            CustomError::InvalidMplCoreProgram
        );

        // 1) Thaw the freeze plugin
        let mut up = UpdatePluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program);
        up.asset(&ctx.accounts.asset)
            .collection(None)
            .payer(&ctx.accounts.payer)
            .system_program(&ctx.accounts.system_program)
            .authority(Some(&ctx.accounts.update_authority))
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }));
        up.invoke()?;

        // 2) Update royalties to post-unlock basis points
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

    pub fn add_plugin(ctx: Context<AddPlugin>, plugin: Plugin) -> Result<()> {
        // Validate core program id
        require_keys_eq!(
            ctx.accounts.mpl_core_program.key(),
            MPL_CORE_ID,
            CustomError::InvalidMplCoreProgram
        );

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
