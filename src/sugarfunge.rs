use crate::{common::TokioRuntime, ipfs::ProofEngine, opts::Opt};
use bevy::prelude::*;
use crossbeam::channel;
use ipfs_api::IpfsApi;
use ipfs_api_backend_hyper as ipfs_api;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time;
use structopt::StructOpt;
use sugarfunge_api_types::{account::*, fula::*, primitives::*, *};

const YEARLY_TOKENS: u64 = 48000000;

const DAILY_TOKENS_MINING: f64 = YEARLY_TOKENS as f64 * 0.70 / (12 * 30) as f64;
const DAILY_TOKENS_STORAGE: f64 = YEARLY_TOKENS as f64 * 0.20 / (12 * 30) as f64;

const NUMBER_CYCLES_TO_ADVANCE: u16 = 24;
const NUMBER_CYCLES_TO_RESET: u16 = 4;

const HOUR_TO_MILISECONDS: u64 = 500; // Should be 3600000 seconds in a day
const YEAR_TO_HOURS: i64 = 360; // Should be 8640 hours in a year

#[derive(Deref)]
pub struct Sender<T>(pub channel::Sender<T>);
#[derive(Deref)]
pub struct Receiver<T>(pub channel::Receiver<T>);

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestError {
    pub message: serde_json::Value,
    pub description: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub peers: usize,
    pub is_syncing: bool,
    pub should_have_peers: bool,
}

fn endpoint(cmd: &'static str) -> String {
    format!("http://127.0.0.1:4000/{}", cmd)
}

async fn req<'a, I, O>(cmd: &'static str, args: I) -> Result<O, RequestError>
where
    I: Serialize,
    O: for<'de> Deserialize<'de>,
{
    let sf_res = reqwest::Client::new()
        .post(endpoint(cmd))
        .json(&args)
        .send()
        .await;

    match sf_res {
        Ok(res) => {
            if let Err(err) = res.error_for_status_ref() {
                match res.json::<RequestError>().await {
                    Ok(err) => Err(err),
                    Err(_) => Err(RequestError {
                        message: json!(format!("{:#?}", err)),
                        description: "Reqwest json error.".into(),
                    }),
                }
            } else {
                match res.json().await {
                    Ok(res) => Ok(res),
                    Err(err) => Err(RequestError {
                        message: json!(format!("{:#?}", err)),
                        description: "Reqwest json error.".into(),
                    }),
                }
            }
        }
        Err(err) => Err(RequestError {
            message: json!(format!("{:#?}", err)),
            description: "Reqwest error.".into(),
        }),
    }
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

async fn get_manifests(
    pool_id: Option<PoolId>,
    uploader: Option<Account>,
    storage: Option<Account>,
) -> Result<GetAllManifestsOutput, RequestError> {
    let manifests: Result<fula::GetAllManifestsOutput, _> = req(
        "fula/manifest",
        fula::GetAllManifestsInput {
            uploader,
            pool_id,
            storage,
        },
    )
    .await;
    return manifests;
}

async fn get_account_pool_id(account: Account) -> Option<PoolId> {
    let user: Result<pool::GetAllPoolUsersOutput, _> = req(
        "fula/pool/users",
        pool::GetAllPoolUsersInput {
            account: Some(account),
        },
    )
    .await;
    if let Ok(ref users) = user {
        let current = users.users.get(0);
        if let Some(user_value) = current {
            return user_value.pool_id;
        } else {
            return None;
        }
    } else {
        return None;
    }
}

pub async fn get_cumulative_size(manifests: &GetAllManifestsOutput) -> u64 {
    //Storage provided by user
    let client = ipfs_api::IpfsClient::default();

    let mut cumulative_size: u64 = 0;

    for value in manifests.manifests.iter() {
        if let Ok(current_manifest) = serde_json::from_value::<crate::manifest::Manifest>(
            value.manifest_data.manifest_metadata.clone(),
        ) {
            if let Ok(_req) = client.pin_ls(Some(&current_manifest.job.uri), None).await {
                if let Ok(file_check) = client.block_stat(&current_manifest.job.uri).await {
                    info!("VERIFICATION✅:  {:#?}", file_check);
                    cumulative_size += file_check.size;
                }
            }
        }
    }
    return cumulative_size;
}

pub async fn get_cumulative_size_proof(peer_id: String) -> u64 {
    //Storage provided by user
    let client = ipfs_api::IpfsClient::default();

    let ipfs_seed = format!("//fula/dev/2/{}", peer_id);
    let seeded = verify_account_seeded(Seed::from(ipfs_seed)).await;

    let manifests = get_manifests(None, None, Some(seeded.account.clone())).await;

    let mut cumulative_size: u64 = 0;

    if let Ok(manifests) = manifests {
        for value in manifests.manifests.iter() {
            if let Ok(current_manifest) = serde_json::from_value::<crate::manifest::Manifest>(
                value.manifest_data.manifest_metadata.clone(),
            ) {
                if let Ok(_req) = client.pin_ls(Some(&current_manifest.job.uri), None).await {
                    if let Ok(file_check) = client.block_stat(&current_manifest.job.uri).await {
                        info!("VERIFICATION✅:  {:#?}", file_check);
                        cumulative_size += file_check.size;
                    }
                }
            }
        }
    }
    return cumulative_size;
}

pub async fn get_blocks_proof(peer_id: String) -> u64 {
    let client = ipfs_api::IpfsClient::default();

    let ipfs_seed = format!("//fula/dev/2/{}", peer_id);
    let seeded = verify_account_seeded(Seed::from(ipfs_seed)).await;

    let manifests = get_manifests(None, None, Some(seeded.account.clone())).await;

    let mut blocks: u64 = 0;

    if let Ok(manifests) = manifests {
        for value in manifests.manifests.iter() {
            if let Ok(current_manifest) = serde_json::from_value::<crate::manifest::Manifest>(
                value.manifest_data.manifest_metadata.clone(),
            ) {
                if let Ok(_req) = client.pin_ls(Some(&current_manifest.job.uri), None).await {
                    if let Ok(_file_check) = client.block_stat(&current_manifest.job.uri).await {
                        blocks += 1;
                    }
                }
            }
        }
    }
    return blocks;
}

async fn verify_account_seeded(seed: Seed) -> account::SeededAccountOutput {
    let seeded: account::SeededAccountOutput =
        req("account/seeded", account::SeededAccountInput { seed })
            .await
            .unwrap();
    return seeded;
}

async fn verify_account_exist(seeded_account: Account) -> bool {
    let account_exists: account::AccountExistsOutput = req(
        "account/exists",
        account::AccountExistsInput {
            account: seeded_account.clone(),
        },
    )
    .await
    .unwrap();
    info!("VERIFICATION:{:?}", account_exists);
    return account_exists.exists;
}

async fn register_account(seeded_account: Account, operator_seed: Seed) {
    if !verify_account_exist(seeded_account.clone()).await {
        let fund: account::FundAccountOutput = req(
            "account/fund",
            account::FundAccountInput {
                seed: operator_seed.clone(),
                to: seeded_account.clone(),
                amount: Balance::from(1000000000000000000),
            },
        )
        .await
        .unwrap();
        if u128::from(fund.amount) > 0 {
            warn!("WARNING: registered: {:?}", seeded_account);
        } else {
            error!("ERROR: could not register account");
        }
    }
}

async fn verify_class_info(class_id: ClassId, operator_seed: Seed, operator_account: Account) {
    let class_info: asset::ClassInfoOutput =
        req("asset/class_info", asset::ClassInfoInput { class_id })
            .await
            .unwrap();

    if class_info.info.is_none() {
        info!("CREATION: creating: {:?}", class_id);
        let create_class: asset::CreateClassOutput = req(
            "asset/create_class",
            asset::CreateClassInput {
                seed: operator_seed.clone(),
                owner: operator_account.clone(),
                class_id,
                metadata: json!({"fula":{"desc": "Proof engine token"}}),
            },
        )
        .await
        .unwrap();
        info!("CREATION: created: {:#?}", create_class);
    }
}

async fn verify_asset_info(class_id: ClassId, asset_id: AssetId, operator_seed: Seed) {
    let asset_info: asset::AssetInfoOutput =
        req("asset/info", asset::AssetInfoInput { class_id, asset_id })
            .await
            .unwrap();

    if asset_info.info.is_none() {
        info!("CREATION: creating: {:?} {:?}", class_id, asset_id);
        let create_asset: asset::CreateOutput = req(
            "asset/create",
            asset::CreateInput {
                seed: operator_seed.clone(),
                class_id,
                asset_id,
                metadata: json!({"ipfs":{"root_hash": "0"}}),
            },
        )
        .await
        .unwrap();
        info!("CREATION: created: {:#?}", create_asset);
    }
}

pub fn launch(sugar_rx: Res<Receiver<ProofEngine>>, tokio_runtime: Res<TokioRuntime>) {
    let rt = tokio_runtime.runtime.clone();

    let sugar_rx: channel::Receiver<ProofEngine> = sugar_rx.clone();

    std::thread::spawn(move || {
        // Spawn the root task
        rt.block_on(async move {
            let proof = sugar_rx.recv().unwrap();
            let cmd_opts = Opt::from_args();
            let class_id = *cmd_opts.class_id;
            let ipfs_seed = format!("//fula/dev/2/{}", &proof.peer_id);
            let asset_id = calculate_hash(&ipfs_seed);
            let asset_id = AssetId::from(asset_id);

            info!(
                "VERIFICATION: ClassId {:?}, AssetId {:?}",
                class_id, asset_id
            );

            //Health Request Loop

            loop {
                let health_request: Result<Health, RequestError> = req("health", ()).await;

                match health_request {
                    Ok(health) => {
                        debug!("VERIFICATION: health: {:#?}", health);
                        break;
                    }
                    Err(err) => {
                        error!("ERROR: health: {:#?}", err);
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                };
            }

            //Verifying the existence of the account, the operator, class_id and asset_id

            let seeded = verify_account_seeded(Seed::from(ipfs_seed)).await;
            info!("VERIFICATION: User Seed {:?}", seeded.seed);
            info!("VERIFICATION: User Account {:?}", seeded.account);

            let operator = verify_account_seeded(cmd_opts.operator.clone()).await;
            info!("VERIFICATION: Operator Seed: {:?}", operator.seed);
            info!("VERIFICATION: Operator Account: {:?}", operator.account);

            register_account(seeded.account.clone(), operator.seed.clone()).await;

            verify_class_info(class_id, operator.seed.clone(), operator.account.clone()).await;

            verify_asset_info(class_id, asset_id, operator.seed.clone()).await;

            //Executing the Calculation, Mint and Update of rewards

            for cycle in 1..YEAR_TO_HOURS {
                let mut daily_rewards = 0.0;
                if let Ok(proof) = sugar_rx.try_recv() {
                    let all_manifests = get_manifests(None, None, None).await;

                    if let Ok(current_all_manifests) = all_manifests {
                        if current_all_manifests.manifests.len() > 0 {
                            //CALCULATE DAILY REWARDS
                            let network_size =
                                get_cumulative_size(&current_all_manifests).await as f64;
                            let rewards = calculate_daily_rewards(network_size, &seeded).await;
                            info!("STEP 1: CALCULATE REWARDS:");
                            info!("  Mining Rewards: {:?}", rewards.daily_mining_rewards);
                            info!("  Mining Rewards: {:?}", rewards.daily_storage_rewards);

                            daily_rewards += rewards.daily_storage_rewards;
                            daily_rewards += rewards.daily_mining_rewards;

                            if daily_rewards > 0.0 {
                                //MINT DAILY REWARDS
                                let mint: asset::MintOutput = match req(
                                    "asset/mint",
                                    asset::MintInput {
                                        seed: operator.seed.clone(),
                                        class_id,
                                        asset_id,
                                        to: seeded.account.clone(),
                                        amount: Balance::from((daily_rewards) as u128),
                                    },
                                )
                                .await
                                {
                                    Ok(mint) => mint,
                                    Err(err) => {
                                        error!("mint: {:#?}", err);
                                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                        continue;
                                    }
                                };
                                info!("STEP 2: MINTED: {:#?}", mint);
                                tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                                //UPDATE METADATA
                                let metadata = json!({"ipfs":{"root_hash": proof.hash}});
                                info!(
                                    "Updating_metadata: {:?} {:?} {:#?}",
                                    class_id, asset_id, metadata
                                );
                                let update_metadata: Result<asset::UpdateMetadataOutput, _> = req(
                                    "asset/update_metadata",
                                    asset::UpdateMetadataInput {
                                        seed: operator.seed.clone(),
                                        class_id,
                                        asset_id,
                                        metadata: metadata.clone(),
                                    },
                                )
                                .await;
                                info!("STEP 3: UPDATED METADATA: {:#?}", update_metadata);
                            } else {
                                info!("STEP 2: NO REWARDS TO BE MINTED");
                            }
                        }
                    }
                }
                println!(
                    "DAY: {} CYCLE: {} REWARDS: {}",
                    cycle / NUMBER_CYCLES_TO_ADVANCE as i64,
                    cycle,
                    daily_rewards
                );
                tokio::time::sleep(time::Duration::from_millis(HOUR_TO_MILISECONDS)).await;
            }
        });
    });
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Rewards {
    pub daily_mining_rewards: f64,
    pub daily_storage_rewards: f64,
}

async fn calculate_daily_rewards(network_size: f64, seeded: &SeededAccountOutput) -> Rewards {
    let mut rewards = Rewards {
        daily_mining_rewards: 0.0,
        daily_storage_rewards: 0.0,
    };
    let pool_id = get_account_pool_id(seeded.account.clone()).await;
    let user_manifests = get_manifests(pool_id, None, Some(seeded.account.clone())).await;
    if let Ok(user_manifests) = user_manifests {
        let user_size = get_cumulative_size(&user_manifests).await as f64;
        let user_participation = user_size / network_size;

        rewards =
            calculate_and_update_storage_rewards(&user_manifests, &seeded, user_participation)
                .await;

        return rewards;
    } else {
        return rewards;
    }
}

pub async fn calculate_and_update_storage_rewards(
    manifests: &GetAllManifestsOutput,
    seeded: &SeededAccountOutput,
    user_participation: f64,
) -> Rewards {
    let mut rewards = Rewards {
        daily_mining_rewards: 0.0,
        daily_storage_rewards: 0.0,
    };

    // let mut cumulative_storage_rewards = 0.0;
    //Storage provided by user
    let client = ipfs_api::IpfsClient::default();

    for manifest in manifests.manifests.iter() {
        let mut updated_data = ManifestStorageData {
            active_cycles: manifest.manifest_storage_data.active_cycles,
            missed_cycles: manifest.manifest_storage_data.missed_cycles,
            active_days: manifest.manifest_storage_data.active_days,
        };
        if let Ok(current_manifest) = serde_json::from_value::<crate::manifest::Manifest>(
            manifest.manifest_data.manifest_metadata.clone(),
        ) {
            if let Ok(_req) = client.pin_ls(Some(&current_manifest.job.uri), None).await {
                // When the active cycles reached {NUMBER_CYCLES_TO_ADVANCE} which is equal to 1 day, the manifest active days are increased and the rewards are calculated
                if manifest.manifest_storage_data.active_cycles == NUMBER_CYCLES_TO_ADVANCE {
                    let active_days = manifest.manifest_storage_data.active_days + 1;

                    // The calculation of the storage rewards
                    rewards.daily_storage_rewards += (1 as f64
                        / (1 as f64 + (-0.1 * (active_days - 45) as f64).exp()))
                        * DAILY_TOKENS_STORAGE
                        * user_participation;

                    rewards.daily_mining_rewards = DAILY_TOKENS_MINING as f64 * user_participation;

                    updated_data.active_days += 1;
                    updated_data.active_cycles = 0;
                } else {
                    updated_data.active_cycles += 1;
                }
            } else {
                // If the verification of the IPFS File failed {NUMBER_CYCLES_TO_RESET} times, the active_days are reset to 0
                if manifest.manifest_storage_data.missed_cycles == NUMBER_CYCLES_TO_RESET {
                    updated_data.missed_cycles = 0;
                    updated_data.active_days = 0;
                } else {
                    // If the times failed are lower, the missed cycles are increased
                    updated_data.missed_cycles += 1;
                }
            }
        }
        // Updated the values of the Storage Data in the Manifest
        let _manifest_updated = updated_storage_data(
            seeded.seed.clone(),
            manifest.manifest_data.manifest_metadata.clone(),
            manifest.pool_id,
            updated_data,
        )
        .await;
    }
    return rewards;
}

async fn updated_storage_data(
    seed: Seed,
    manifest_metadata: serde_json::Value,
    pool_id: PoolId,
    manifest_storage_data: ManifestStorageData,
) -> Result<ManifestUpdatedOutput, RequestError> {
    let manifest: Result<fula::ManifestUpdatedOutput, _> = req(
        "fula/manifest",
        fula::UpdateManifestInput {
            seed,
            manifest_metadata,
            pool_id,
            active_days: manifest_storage_data.active_days,
            active_cycles: manifest_storage_data.active_cycles,
            missed_cycles: manifest_storage_data.missed_cycles,
        },
    )
    .await;
    return manifest;
}
