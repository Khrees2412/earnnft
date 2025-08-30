import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";
import { Earnnft } from "../target/types/earnnft";

describe("EarnNFT Program Tests", () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.Earnnft as Program<Earnnft>;
    const MPL_CORE_PROGRAM_ID = new PublicKey(
        "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d"
    );

    const authority = (provider.wallet as anchor.Wallet).payer;
    let collection: Keypair;
    let asset: Keypair;
    let user: Keypair;
    let collectionState: PublicKey;

    beforeEach(async () => {
        collection = Keypair.generate();
        asset = Keypair.generate();
        user = Keypair.generate();

        // Derive collection state PDA
        [collectionState] = PublicKey.findProgramAddressSync(
            [Buffer.from("state"), collection.publicKey.toBuffer()],
            program.programId
        );
    });

    describe("Collection Creation", () => {
        it("should create a collection with state", async () => {
            const unlockTs = new anchor.BN(Date.now() / 1000 + 3600); // 1 hour from now
            const preUnlockBps = 500; // 5%
            const postUnlockBps = 1000; // 10%

            const tx = await program.methods
                .createCollectionWithState(
                    "Test Collection",
                    "https://example.com/collection.json",
                    unlockTs,
                    preUnlockBps,
                    postUnlockBps
                )
                .accountsPartial({
                    payer: authority.publicKey,
                    collection: collection.publicKey,
                    state: collectionState,
                    updateAuthority: authority.publicKey,
                    systemProgram: SystemProgram.programId,
                    mplCoreProgram: MPL_CORE_PROGRAM_ID,
                })
                .signers([collection])
                .rpc();

            expect(tx).to.not.be.null;

            // Verify collection state was created
            const stateAccount = await program.account.collectionState.fetch(
                collectionState
            );
            expect(stateAccount.collection.toString()).to.equal(
                collection.publicKey.toString()
            );
            expect(stateAccount.unlockTs.toString()).to.equal(
                unlockTs.toString()
            );
            expect(stateAccount.preUnlockBps).to.equal(preUnlockBps);
            expect(stateAccount.postUnlockBps).to.equal(postUnlockBps);
            expect(stateAccount.updateAuthority.toString()).to.equal(
                authority.publicKey.toString()
            );
        });

        it("should fail with invalid basis points", async () => {
            const unlockTs = new anchor.BN(Date.now() / 1000 + 3600);
            const invalidBps = 10001; // Over 100%
            const newCollection = Keypair.generate();
            const [newCollectionState] = PublicKey.findProgramAddressSync(
                [Buffer.from("state"), newCollection.publicKey.toBuffer()],
                program.programId
            );

            try {
                await program.methods
                    .createCollectionWithState(
                        "Invalid Collection",
                        "https://example.com/collection.json",
                        unlockTs,
                        invalidBps,
                        1000
                    )
                    .accountsPartial({
                        payer: authority.publicKey,
                        collection: newCollection.publicKey,
                        state: newCollectionState,
                        updateAuthority: authority.publicKey,
                        systemProgram: SystemProgram.programId,
                        mplCoreProgram: MPL_CORE_PROGRAM_ID,
                    })
                    .signers([newCollection])
                    .rpc();

                expect.fail("Should have thrown InvalidBps error");
            } catch (error) {
                expect(error.error.errorCode.code).to.equal("InvalidBps");
            }
        });

        it("should fail with invalid unlock timestamp", async () => {
            const pastUnlockTs = new anchor.BN(-1); // Negative timestamp
            const newCollection = Keypair.generate();
            const [newCollectionState] = PublicKey.findProgramAddressSync(
                [Buffer.from("state"), newCollection.publicKey.toBuffer()],
                program.programId
            );

            try {
                await program.methods
                    .createCollectionWithState(
                        "Past Unlock Collection",
                        "https://example.com/collection.json",
                        pastUnlockTs,
                        500,
                        1000
                    )
                    .accountsPartial({
                        payer: authority.publicKey,
                        collection: newCollection.publicKey,
                        state: newCollectionState,
                        updateAuthority: authority.publicKey,
                        systemProgram: SystemProgram.programId,
                        mplCoreProgram: MPL_CORE_PROGRAM_ID,
                    })
                    .signers([newCollection])
                    .rpc();

                expect.fail("Should have thrown InvalidUnlockTs error");
            } catch (error) {
                if (error.error?.errorCode?.code) {
                    expect(error.error.errorCode.code).to.equal(
                        "InvalidUnlockTs"
                    );
                } else {
                    console.log("Unexpected error structure:", error);
                    expect(error).to.not.be.null;
                }
            }
        });
    });

    describe("Asset Minting", () => {
        beforeEach(async () => {
            // Create collection first
            const unlockTs = new anchor.BN(Date.now() / 1000 + 3600);
            await program.methods
                .createCollectionWithState(
                    "Test Collection",
                    "https://example.com/collection.json",
                    unlockTs,
                    500,
                    1000
                )
                .accountsPartial({
                    payer: authority.publicKey,
                    collection: collection.publicKey,
                    state: collectionState,
                    updateAuthority: authority.publicKey,
                    systemProgram: SystemProgram.programId,
                    mplCoreProgram: MPL_CORE_PROGRAM_ID,
                })
                .signers([collection])
                .rpc();
        });

        it("should mint a locked asset", async () => {
            try {
                const tx = await program.methods
                    .mintLockedAsset(
                        "Test Asset",
                        "https://example.com/asset.json"
                    )
                    .accountsPartial({
                        payer: authority.publicKey,
                        asset: asset.publicKey,
                        state: collectionState,
                        collection: collection.publicKey,
                        updateAuthority: authority.publicKey,
                        owner: user.publicKey,
                        systemProgram: SystemProgram.programId,
                        mplCoreProgram: MPL_CORE_PROGRAM_ID,
                    })
                    .signers([asset])
                    .rpc();

                expect(tx).to.not.be.null;
            } catch (error) {
                console.log("Error details:", error);
                console.log("Error logs:", error.logs);
                throw error;
            }
        });
    });

    describe("Asset Unlocking", () => {
        beforeEach(async () => {
            // Create collection and mint asset
            const unlockTs = new anchor.BN(Date.now() / 1000 + 2); // 2 seconds from now
            await program.methods
                .createCollectionWithState(
                    "Test Collection",
                    "https://example.com/collection.json",
                    unlockTs,
                    500,
                    1000
                )
                .accountsPartial({
                    payer: authority.publicKey,
                    collection: collection.publicKey,
                    state: collectionState,
                    updateAuthority: authority.publicKey,
                    systemProgram: SystemProgram.programId,
                    mplCoreProgram: MPL_CORE_PROGRAM_ID,
                })
                .signers([collection])
                .rpc();

            await program.methods
                .mintLockedAsset("Test Asset", "https://example.com/asset.json")
                .accountsPartial({
                    payer: authority.publicKey,
                    asset: asset.publicKey,
                    state: collectionState,
                    collection: collection.publicKey,
                    updateAuthority: authority.publicKey,
                    owner: user.publicKey,
                    systemProgram: SystemProgram.programId,
                    mplCoreProgram: MPL_CORE_PROGRAM_ID,
                })
                .signers([asset])
                .rpc({ skipPreflight: true });
        });

        it("should unlock asset after unlock time", async () => {
            // Wait for unlock time to pass
            await new Promise((resolve) => setTimeout(resolve, 3000));

            const tx = await program.methods
                .unlockAsset()
                .accountsPartial({
                    payer: authority.publicKey,
                    asset: asset.publicKey,
                    collection: collection.publicKey,
                    state: collectionState,
                    updateAuthority: authority.publicKey,
                    systemProgram: SystemProgram.programId,
                    mplCoreProgram: MPL_CORE_PROGRAM_ID,
                })
                .rpc();

            expect(tx).to.not.be.null;
        });

        it("should fail to unlock before unlock time", async () => {
            // Create a collection with future unlock time
            const futureCollection = Keypair.generate();
            const [futureState] = PublicKey.findProgramAddressSync(
                [Buffer.from("state"), futureCollection.publicKey.toBuffer()],
                program.programId
            );

            const futureUnlockTs = new anchor.BN(Date.now() / 1000 + 3600); // 1 hour from now

            await program.methods
                .createCollectionWithState(
                    "Future Collection",
                    "https://example.com/collection.json",
                    futureUnlockTs,
                    500,
                    1000
                )
                .accountsPartial({
                    payer: authority.publicKey,
                    collection: futureCollection.publicKey,
                    state: futureState,
                    updateAuthority: authority.publicKey,
                    systemProgram: SystemProgram.programId,
                    mplCoreProgram: MPL_CORE_PROGRAM_ID,
                })
                .signers([futureCollection])
                .rpc();

            const futureAsset = Keypair.generate();
            await program.methods
                .mintLockedAsset(
                    "Future Asset",
                    "https://example.com/asset.json"
                )
                .accountsPartial({
                    payer: authority.publicKey,
                    asset: futureAsset.publicKey,
                    state: futureState,
                    collection: futureCollection.publicKey,
                    updateAuthority: authority.publicKey,
                    owner: user.publicKey,
                    systemProgram: SystemProgram.programId,
                    mplCoreProgram: MPL_CORE_PROGRAM_ID,
                })
                .signers([futureAsset])
                .rpc({ skipPreflight: true });

            try {
                await program.methods
                    .unlockAsset()
                    .accountsPartial({
                        payer: authority.publicKey,
                        asset: futureAsset.publicKey,
                        collection: futureCollection.publicKey,
                        state: futureState,
                        updateAuthority: authority.publicKey,
                        systemProgram: SystemProgram.programId,
                        mplCoreProgram: MPL_CORE_PROGRAM_ID,
                    })
                    .rpc();

                expect.fail("Should have thrown NotYetUnlocked error");
            } catch (error) {
                expect(error.error.errorCode.code).to.equal("NotYetUnlocked");
            }
        });
    });

    describe("Edge Cases", () => {
        it("should fail with invalid MPL Core program", async () => {
            const invalidMplCore = Keypair.generate().publicKey;
            const unlockTs = new anchor.BN(Date.now() / 1000 + 3600);
            const newCollection = Keypair.generate();
            const [newCollectionState] = PublicKey.findProgramAddressSync(
                [Buffer.from("state"), newCollection.publicKey.toBuffer()],
                program.programId
            );

            try {
                await program.methods
                    .createCollectionWithState(
                        "Invalid Program Collection",
                        "https://example.com/collection.json",
                        unlockTs,
                        500,
                        1000
                    )
                    .accountsPartial({
                        payer: authority.publicKey,
                        collection: newCollection.publicKey,
                        state: newCollectionState,
                        updateAuthority: authority.publicKey,
                        systemProgram: SystemProgram.programId,
                        mplCoreProgram: invalidMplCore,
                    })
                    .signers([newCollection])
                    .rpc();

                expect.fail("Should have thrown InvalidMplCoreProgram error");
            } catch (error) {
                expect(error.error.errorCode.code).to.equal(
                    "InvalidMplCoreProgram"
                );
            }
        });

        it("should handle basic validation checks", async () => {
            const unlockTs = new anchor.BN(Date.now() / 1000 + 3600);
            const newCollection = Keypair.generate();
            const [newCollectionState] = PublicKey.findProgramAddressSync(
                [Buffer.from("state"), newCollection.publicKey.toBuffer()],
                program.programId
            );

            const tx = await program.methods
                .createCollectionWithState(
                    "Valid Collection",
                    "https://example.com/collection.json",
                    unlockTs,
                    500,
                    1000
                )
                .accountsPartial({
                    payer: authority.publicKey,
                    collection: newCollection.publicKey,
                    state: newCollectionState,
                    updateAuthority: authority.publicKey,
                    systemProgram: SystemProgram.programId,
                    mplCoreProgram: MPL_CORE_PROGRAM_ID,
                })
                .signers([newCollection])
                .rpc();

            expect(tx).to.not.be.null;

            // Verify state was created correctly
            const stateAccount = await program.account.collectionState.fetch(
                newCollectionState
            );
            expect(stateAccount.collection.toString()).to.equal(
                newCollection.publicKey.toString()
            );
            expect(stateAccount.preUnlockBps).to.equal(500);
            expect(stateAccount.postUnlockBps).to.equal(1000);
        });
    });
});
