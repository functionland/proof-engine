use crate::{common::TokioRuntime, ipfs::ProofEngine, opts::Opt};
use bevy::prelude::*;
use crossbeam::channel;
use ipfs_api::IpfsApi;
use ipfs_api_backend_hyper as ipfs_api;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use structopt::StructOpt;
use sugarfunge_api_types::{fula::*, primitives::*, *};

const GB: u32 = 1048576;
const USER_STORAGE_PROVIDED: u16 = 1000; //GBs
const TOTAL_USERS: u16 = 2000;
const NETWORK_SIZE: u64 = USER_STORAGE_PROVIDED as u64 * TOTAL_USERS as u64;
const USER_PARTICIPATION: f32 = USER_STORAGE_PROVIDED as f32 / NETWORK_SIZE as f32; // 0,0005 ; Percentage
const YEARLY_TOKENS: u64 = 48000000;
const DAILY_TOKENS_MINING: f64 = YEARLY_TOKENS as f64 *0.70 /(12*30) as f64; //93333.333
// const DAILY_TOKENS_STORAGE: f64 = YEARLY_TOKENS as f64 *0.20 /(12*30) as f64; //26666,666
// const DAILY_TOKENS_STORAGE_REWARDS: f32 = DAILY_TOKENS_STORAGE as f32 * USER_PARTICIPATION; //13.333
const DAILY_TOKENS_MINING_REWARDS: f32 = DAILY_TOKENS_MINING as f32 * USER_PARTICIPATION; // 46.667

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

async fn get_manifests(storer: Account) -> Result<GetAllManifestsOutput, RequestError> {
    let storage = storer;

    let manifests: Result<fula::GetAllManifestsOutput, _> = req(
        "fula/manifest",
        fula::GetAllManifestsInput {
            uploader: None,
            pool_id: None,
            storage: Some(storage) 
        },
    )
    .await;
    return manifests;
}

pub async fn get_cumulative_size_proof(peer_id: String) -> u64 { //Storage provided by user
    let client = ipfs_api::IpfsClient::default();

    let ipfs_seed = format!("//fula/dev/2/{}", peer_id);
    let seeded = verify_account_seeded(Seed::from(ipfs_seed)).await;

    let manifests = get_manifests(seeded.account.clone()).await;

    let mut cumulative_size: u64 = 0;

    if let Ok(manifests) = manifests {
            for value in manifests.manifests.iter() {
                if let Ok(current_manifest) = serde_json::from_value::<crate::manifest::Manifest>(value.manifest_data.manifest_metadata.clone()){

                    if let Ok(_req) = client.pin_ls(Some(&current_manifest.job.uri), None).await{
                        if let Ok(file_check) = client.block_stat(&current_manifest.job.uri).await {
                            info!("✅:  {:#?}", file_check);
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

    let manifests = get_manifests(seeded.account.clone()).await;

    let mut blocks: u64 = 0;

    if let Ok(manifests) = manifests {
        for value in manifests.manifests.iter() {
            if let Ok(current_manifest) = serde_json::from_value::<crate::manifest::Manifest>(value.manifest_data.manifest_metadata.clone()){

                if let Ok(_req) = client.pin_ls(Some(&current_manifest.job.uri), None).await{
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

async fn verify_account_exist(seeded_account: Account) -> bool{
    let account_exists: account::AccountExistsOutput = req(
        "account/exists",
        account::AccountExistsInput {
            account: seeded_account.clone(),
        },
    )
    .await
    .unwrap();
    info!("{:?}", account_exists);
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
            warn!("registered: {:?}", seeded_account);
        } else {
            error!("could not register account");
        }
    }
}

async fn verify_class_info(class_id: ClassId, operator_seed: Seed, operator_account: Account) {
    let class_info: asset::ClassInfoOutput =
        req("asset/class_info", asset::ClassInfoInput { class_id })
            .await
            .unwrap();

    if class_info.info.is_none() {
        info!("creating: {:?}", class_id);
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
        info!("created: {:#?}", create_class);
    }
}

async fn verify_asset_info(class_id: ClassId, asset_id: AssetId, operator_seed: Seed) {
    let asset_info: asset::AssetInfoOutput =
        req("asset/info", asset::AssetInfoInput { class_id, asset_id })
            .await
            .unwrap();

    if asset_info.info.is_none() {
        info!("creating: {:?} {:?}", class_id, asset_id);
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
        info!("created: {:#?}", create_asset);
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

            //let mut day_count = 0;

            info!("{:?} {:?}", class_id, asset_id);

            //Health Request Loop

            loop {
                let health_request: Result<Health, RequestError> = req("health", ()).await;

                match health_request {
                    Ok(health) => {
                        debug!("health: {:#?}", health);
                        break;
                    }
                    Err(err) => {
                        error!("health: {:#?}", err);
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                };
            }

            //Verifying the existence of the account, the operator, class_id and asset_id

            let seeded = verify_account_seeded(Seed::from(ipfs_seed)).await;
            info!("{:?}", seeded.seed);
            info!("{:?}", seeded.account);

            let operator = verify_account_seeded(cmd_opts.operator.clone()).await;
            info!("operator: {:?}", operator.seed);
            info!("operator: {:?}", operator.account);

            register_account(seeded.account.clone(), operator.seed.clone()).await;

            verify_account_exist(seeded.account.clone()).await;

            verify_class_info(class_id, operator.seed.clone(), operator.account.clone()).await;

            verify_asset_info(class_id, asset_id, operator.seed.clone()).await;

            //get_manifests(seeded.account.clone()).await;

            //Executing the Calculation, Mint and Update of rewards

            loop {
                if let Ok(proof) = sugar_rx.try_recv() {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await; //adjust to imitate days
                    //day_count+=1;

                    let manifests =
                        get_manifests(seeded.account.clone()).await;

                    if let Ok(_) = manifests {
                        info!("mint for: {:#?}", proof);

                        //Mining Rewards:
                        let daily_mining_rewards= proof.cumulative_size as f32 * DAILY_TOKENS_MINING_REWARDS / (1024 * GB) as f32; 

                        //Storage Rewards:
                        //let daily_storage_rewards = (1 as f32 / (1 as f32 + (-0.1*(day_count-45) as f32).exp())) * DAILY_TOKENS_STORAGE_REWARDS; //Daily Income Calculation

                        //Mint Daily Rewards
                        let mint: asset::MintOutput = match req(
                            "asset/mint",
                            asset::MintInput {
                                seed: operator.seed.clone(),
                                class_id,
                                asset_id,
                                to: seeded.account.clone(),
                                amount: Balance::from((daily_mining_rewards) as u128),
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
                        info!("{:#?}", mint);
                        
                        info!("Mining Rewards: {:#?}", daily_mining_rewards);

                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                        let metadata = json!({"ipfs":{"root_hash": proof.hash}});

                        info!(
                            "updating_metadata: {:?} {:?} {:#?}",
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
                        info!("{:#?}", update_metadata);
                    }
                }
            }
        });
    });
}
