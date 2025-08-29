import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Earnnft } from "../target/types/earnnft";
import { Keypair, SystemProgram, PublicKey } from "@solana/web3.js";
import {
    MPL_CORE_PROGRAM_ID as MPL_CORE_ID,
    fetchAsset,
} from "@metaplex-foundation/mpl-core";
import { assert } from "chai";

describe("earnnft", () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.Earnnft as Program<Earnnft>;
    const payer = provider.wallet as anchor.Wallet;

    // Keypairs
    const collection = Keypair.generate();
    const updateAuthority = Keypair.generate();
    const asset = Keypair.generate();

    // PDAs
    const [statePda] = PublicKey.findProgramAddressSync(
        [Buffer.from("state"), collection.publicKey.toBuffer()],
        program.programId
    );

    // Test constants
    const COLLECTION_NAME = "Test Collection";
    const COLLECTION_URI = "https://test.com/collection.json";
    const ASSET_NAME = "Test Asset";
    const ASSET_URI = "https://test.com/asset.json";
    const PRE_UNLOCK_BPS = 500; // 5%
    const POST_UNLOCK_BPS = 100; // 1%
    const UNLOCK_SECONDS = 2; // 2 seconds from now

    it("Creates a new collection with state", async () => {
        const now = Math.floor(Date.now() / 1000);
        const unlockTs = new anchor.BN(now + UNLOCK_SECONDS);

        await program.methods
            .createCollectionWithState(
                COLLECTION_NAME,
                COLLECTION_URI,
                unlockTs,
                PRE_UNLOCK_BPS,
                POST_UNLOCK_BPS
            )
            .accountsPartial({
                payer: payer.publicKey,
                collection: collection.publicKey,
                state: statePda,
                updateAuthority: updateAuthority.publicKey,
                systemProgram: SystemProgram.programId,
                mplCoreProgram: MPL_CORE_ID,
            })
            .signers([collection, payer.payer])
            .rpc();

        // Assert state PDA was created correctly
        const stateAccount = await program.account.collectionState.fetch(
            statePda
        );
        assert.ok(stateAccount.collection.equals(collection.publicKey));
        assert.ok(
            stateAccount.updateAuthority.equals(updateAuthority.publicKey)
        );
        assert.equal(stateAccount.preUnlockBps, PRE_UNLOCK_BPS);
        assert.equal(stateAccount.postUnlockBps, POST_UNLOCK_BPS);
        assert.ok(stateAccount.unlockTs.eq(unlockTs));

        // Assert Core Collection was created correctly
        const collectionAsset = await fetchAsset(
            provider.connection,
            collection.publicKey
        );
        assert.equal(collectionAsset.name, COLLECTION_NAME);
        assert.equal(collectionAsset.uri, COLLECTION_URI);
        assert.ok(
            collectionAsset.updateAuthority.equals(updateAuthority.publicKey)
        );
    });

    it("Mints a new locked asset", async () => {
        await program.methods
            .mintLockedAsset(ASSET_NAME, ASSET_URI)
            .accountsPartial({
                payer: payer.publicKey,
                asset: asset.publicKey,
                state: statePda,
                collection: collection.publicKey,
                updateAuthority: updateAuthority.publicKey,
                owner: payer.publicKey,
                systemProgram: SystemProgram.programId,
                mplCoreProgram: MPL_CORE_ID,
            })
            .signers([asset, payer.payer])
            .rpc();

        // Assert Core Asset was created correctly and is locked
        const mintedAsset = await fetchAsset(
            provider.connection,
            asset.publicKey
        );
        assert.equal(mintedAsset.name, ASSET_NAME);
        assert.ok(
            mintedAsset.updateAuthority.equals(updateAuthority.publicKey)
        );

        // Check for FreezeDelegate plugin
        const freezePlugin = mintedAsset.plugins.find(
            (p) => p.type === "FreezeDelegate"
        );
        assert.isDefined(freezePlugin, "FreezeDelegate plugin not found");
        assert.isTrue(freezePlugin.frozen, "Asset should be frozen");

        // Check for Royalties plugin
        const royaltiesPlugin = mintedAsset.plugins.find(
            (p) => p.type === "Royalties"
        );
        assert.isDefined(royaltiesPlugin, "Royalties plugin not found");
        assert.equal(
            royaltiesPlugin.basisPoints,
            PRE_UNLOCK_BPS,
            "Royalties should be pre-unlock bps"
        );
    });

    it("Fails to unlock the asset before the unlock timestamp", async () => {
        try {
            await program.methods
                .unlockAsset()
                .accountsPartial({
                    payer: payer.publicKey,
                    asset: asset.publicKey,
                    state: statePda,
                    updateAuthority: updateAuthority.publicKey,
                    systemProgram: SystemProgram.programId,
                    mplCoreProgram: MPL_CORE_ID,
                })
                .signers([updateAuthority])
                .rpc();
            assert.fail("Transaction should have failed");
        } catch (err) {
            assert.include(err.message, "NotYetUnlocked");
        }
    });

    it("Successfully unlocks the asset after the unlock timestamp", async () => {
        // Wait for the unlock time to pass
        console.log(
            `Waiting ${UNLOCK_SECONDS} seconds for unlock timestamp to pass...`
        );
        await new Promise((resolve) =>
            setTimeout(resolve, UNLOCK_SECONDS * 1000)
        );

        await program.methods
            .unlockAsset()
            .accountsPartial({
                payer: payer.publicKey,
                asset: asset.publicKey,
                state: statePda,
                updateAuthority: updateAuthority.publicKey,
                systemProgram: SystemProgram.programId,
                mplCoreProgram: MPL_CORE_ID,
            })
            .signers([updateAuthority])
            .rpc();

        // Assert asset is now unlocked and royalties are updated
        const unlockedAsset = await fetchAsset(
            provider.connection,
            asset.publicKey
        );

        // Check FreezeDelegate plugin is thawed
        const freezePlugin = unlockedAsset.plugins.find(
            (p) => p.type === "FreezeDelegate"
        );
        assert.isDefined(freezePlugin, "FreezeDelegate plugin not found");
        assert.isFalse(
            freezePlugin.frozen,
            "Asset should be thawed (not frozen)"
        );

        // Check Royalties plugin is updated
        const royaltiesPlugin = unlockedAsset.plugins.find(
            (p) => p.type === "Royalties"
        );
        assert.isDefined(royaltiesPlugin, "Royalties plugin not found");
        assert.equal(
            royaltiesPlugin.basisPoints,
            POST_UNLOCK_BPS,
            "Royalties should be post-unlock bps"
        );
    });
});
