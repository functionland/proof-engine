use crate::{common::TokioRuntime, config, opts::Opt};
use bevy::prelude::*;
use contract_api_types::{config::Config, calls::{RefundInput, RefundOutput, ConvertToValidatorInput, ConvertToValidatorOutput}};
use crossbeam::channel;
use dotenv::dotenv;
use ipfs_api::IpfsApi;
use ipfs_api_backend_hyper as ipfs_api;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time;
use structopt::StructOpt;
use sugarfunge_api_types::{account::*, challenge::*, fula::*, primitives::*, *};

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
    #[serde(rename = "is_syncing")]
    pub is_syncing: bool,
    #[serde(rename = "should_have_peers")]
    pub should_have_peers: bool,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Rewards {
    pub daily_mining_rewards: f64,
    pub daily_storage_rewards: f64,
}
fn endpoint(host: String, cmd: &'static str) -> String {
    format!("{}/{}", host.as_str(), cmd)
}

async fn req<'a, I, O>(host: String, cmd: &'static str, args: I) -> Result<O, RequestError>
where
    I: Serialize,
    O: for<'de> Deserialize<'de>,
{
    let sf_res = reqwest::Client::new()
        .post(endpoint(host, cmd))
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

async fn fula_sugarfunge_req<'a, I, O>(cmd: &'static str, args: I) -> Result<O, RequestError>
where
    I: Serialize,
    O: for<'de> Deserialize<'de>,
{
    dotenv().ok();
    let env = config::init();
    req(env.fula_sugarfunge_api_host, cmd, args).await
}

async fn fula_contract_req<'a, I, O>(cmd: &'static str, args: I) -> Result<O, RequestError>
where
    I: Serialize,
    O: for<'de> Deserialize<'de>,
{
    dotenv().ok();
    let env = config::init();
    req(env.fula_contract_api_host, cmd, args).await
}

async fn get_manifests(
    pool_id: Option<PoolId>,
    uploader: Option<Account>,
    storage: Option<Account>,
) -> Result<GetAllManifestsOutput, RequestError> {
    let manifests: Result<fula::GetAllManifestsOutput, _> = fula_sugarfunge_req(
        "fula/manifest",
        fula::GetAllManifestsInput {
            uploader,
            pool_id,
            storer: storage,
        },
    )
    .await;
    return manifests;
}

async fn get_manifests_storage_data(
    pool_id: Option<PoolId>,
    storer: Option<Account>,
) -> Result<GetAllManifestsStorerDataOutput, RequestError> {
    let manifests: Result<fula::GetAllManifestsStorerDataOutput, _> = fula_sugarfunge_req(
        "fula/manifest/storer_data",
        fula::GetAllManifestsStorerDataInput { pool_id, storer },
    )
    .await;
    return manifests;
}

async fn get_account_pool_id(account: Account) -> Option<PoolId> {
    let user: Result<pool::GetAllPoolUsersOutput, _> = fula_sugarfunge_req(
        "fula/pool/users",
        pool::GetAllPoolUsersInput {
            account: Some(account),
            pool_id: None,
            request_pool_id: None,
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

pub fn calculate_total_storers(uploaders: Vec<UploaderData>) -> u64 {
    let mut total = 0;

    for uploader in uploaders {
        total += uploader.storers.len() as u64
    }

    return total;
}

pub fn get_vec_cid_from_manifest_storer_data(data: Vec<ManifestStorageData>) -> Vec<Cid> {
    let mut vec_cids = Vec::<Cid>::new();

    for manifest in data {
        vec_cids.push(manifest.cid)
    }

    return vec_cids;
}

pub async fn get_cumulative_size(manifests: &GetAllManifestsOutput) -> u64 {
    //Storage provided by user
    let client = ipfs_api::IpfsClient::default();

    let mut cumulative_size: u64 = 0;

    for value in manifests.manifests.iter() {
        if let Ok(current_manifest) =
            serde_json::from_value::<crate::manifest::Manifest>(value.manifest_metadata.clone())
        {
            if let Ok(_req) = client.pin_ls(Some(&current_manifest.job.uri), None).await {
                if let Ok(file_check) = client.block_stat(&current_manifest.job.uri).await {
                    info!("VERIFICATION✅:  {:#?}", file_check);

                    cumulative_size +=
                        file_check.size * calculate_total_storers(value.uploaders.to_vec());
                }
            }
        }
    }
    return cumulative_size;
}

pub async fn get_file_sizes(cids: Vec<Cid>) -> (Vec<Cid>, Vec<u64>) {
    //Storage provided by user
    let client = ipfs_api::IpfsClient::default();

    let mut vec_sizes: Vec<u64> = Vec::<u64>::new();

    for cid in cids.iter() {
        if let Ok(_req) = client.pin_ls(Some(cid.as_str()), None).await {
            if let Ok(file_check) = client.block_stat(cid.as_str()).await {
                // info!("VERIFICATION✅:  {:#?}", file_check);

                vec_sizes.push(file_check.size)
            }
        }
    }
    return (cids.to_vec(), vec_sizes);
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
            if let Ok(current_manifest) =
                serde_json::from_value::<crate::manifest::Manifest>(value.manifest_metadata.clone())
            {
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
            if let Ok(current_manifest) =
                serde_json::from_value::<crate::manifest::Manifest>(value.manifest_metadata.clone())
            {
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
        fula_sugarfunge_req("account/seeded", account::SeededAccountInput { seed })
            .await
            .unwrap();
    return seeded;
}

async fn verify_account_exist(seeded_account: Account) -> bool {
    let account_exists: account::AccountExistsOutput = fula_sugarfunge_req(
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

async fn register_account(seeded_account: Account) {
    if !verify_account_exist(seeded_account.clone()).await {
        error!("ERROR: Account is not created or funded");
    }
}

async fn verify_class_info(class_id: ClassId, seeded_seed: Seed, seeded_account: Account) {
    let class_info: asset::ClassInfoOutput =
        fula_sugarfunge_req("asset/class_info", asset::ClassInfoInput { class_id })
            .await
            .unwrap();

    if class_info.info.is_none() {
        info!("CREATION: creating: {:?}", class_id);
        let create_class: asset::CreateClassOutput = fula_sugarfunge_req(
            "asset/create_class",
            asset::CreateClassInput {
                seed: seeded_seed.clone(),
                owner: seeded_account.clone(),
                class_id,
                metadata: json!({"fula":{"desc": "Proof engine token"}}),
            },
        )
        .await
        .unwrap();
        info!("CREATION: created: {:#?}", create_class);
    }
}

async fn verify_asset_info(class_id: ClassId, asset_id: AssetId, seeded_seed: Seed) {
    let asset_info: asset::AssetInfoOutput =
        fula_sugarfunge_req("asset/info", asset::AssetInfoInput { class_id, asset_id })
            .await
            .unwrap();

    if asset_info.info.is_none() {
        info!("CREATION: creating: {:?} {:?}", class_id, asset_id);
        let create_asset: asset::CreateOutput = fula_sugarfunge_req(
            "asset/create",
            asset::CreateInput {
                seed: seeded_seed.clone(),
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

async fn fund_account(seeded_account: Account) {
    let data = refund(RefundInput{account: seeded_account.to_string()}).await;
    if let Ok(response) = data {
        info!("CREATION: Account funded: {:#?}", response);
    }
}

pub fn launch(tokio_runtime: Res<TokioRuntime>) {
    let rt = tokio_runtime.runtime.clone();

    std::thread::spawn(move || {
        // Spawn the root task
        rt.block_on(async move {
            let cmd_opts = Opt::from_args();

            if let Ok (config) = setup().await {
                let class_id_labor = ClassId::from(config.labor_token_class_id);
                let asset_id_labor = AssetId::from(config.labor_token_class_id);
                let class_id_challenge = ClassId::from(config.challenge_token_class_id);
                let asset_id_challenge = AssetId::from(config.challenge_token_asset_id);

                //Health Request Loop

                loop {
                    let health_request: Result<Health, RequestError> =
                        fula_sugarfunge_req("health", ()).await;

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

                //Verifying the existence of the account, class_id and asset_id

                info!(
                    "VERIFICATION: ClassId {:?}, AssetId {:?}",
                    class_id_labor, asset_id_labor
                );

                let seeded = verify_account_seeded(cmd_opts.seed.clone()).await;
                info!("VERIFICATION: User Seed {:?}", seeded.seed);
                info!("VERIFICATION: User Account {:?}", seeded.account);

                fund_account(seeded.account.clone()).await;
                
                register_account(seeded.account.clone()).await;

                if let Ok(_) = convert_to_validator(ConvertToValidatorInput { seed: seeded.seed.to_string(), aura_account: cmd_opts.aura.to_string(), grandpa_account: cmd_opts.grandpa.to_string() }).await{
                    verify_class_info(class_id_labor, seeded.seed.clone(), seeded.account.clone()).await;

                    verify_asset_info(class_id_labor, asset_id_labor, seeded.seed.clone()).await;
    
                    verify_class_info(
                        class_id_challenge,
                        seeded.seed.clone(),
                        seeded.account.clone(),
                    )
                    .await;
    
                    verify_asset_info(class_id_challenge, asset_id_challenge, seeded.seed.clone()).await;
    
                    //Executing the Calculation, Mint and Update of rewards
    
                    for cycle in 1..config.total_cyles {
                        let mut daily_rewards = 0.0;
    
                        // Get the current pool_id of the user
                        let pool_id = get_account_pool_id(seeded.account.clone()).await;
    
                        // Validate the manifests of the user on-chain, also the invalid manifests are going to be remove from the chain
                        let user_manifests = validate_current_manifests(VerifyManifestsInput {
                            seed: seeded.seed.clone(),
                        })
                        .await;
                        info!("STEP 1: VERIFY USER MANIFESTS {:#?}", user_manifests);
    
                        // Verify if there is a file in the chain storage that the user is storaging where the size is unknown on-chain
                        if let Ok(verify_file_size_response) = verify_file_size(VerifyFileSizeInput {
                            account: seeded.account.clone(),
                        })
                        .await
                        {
                            if verify_file_size_response.cids.len() > 0 {
                                let result = get_file_sizes(verify_file_size_response.cids).await;
                                if let Some(pool_id) = pool_id {
                                    let _result = provide_file_size(ProvideFileSizeInput {
                                        seed: seeded.seed.clone(),
                                        pool_id,
                                        cids: result.0,
                                        sizes: result.1,
                                    })
                                    .await;
                                };
                            }
                        }
    
                        // Verify if there is an Open challenge for the user, if there is a challenge verify the cid content of the user
                        if let Ok(verify_pending_challenge_response) =
                            verify_pending_challenge(VerifyPendingChallengeInput {
                                account: seeded.account.clone(),
                            })
                            .await
                        {
                            if verify_pending_challenge_response.pending {
                                if let Some(pool_id) = pool_id {
                                    if let Ok(value) = get_manifests_storage_data(
                                        Some(pool_id),
                                        Some(seeded.account.clone()),
                                    )
                                    .await
                                    {
                                        let _result = verify_challenge(VerifyChallengeInput {
                                            seed: seeded.seed.clone(),
                                            pool_id: pool_id,
                                            cids: get_vec_cid_from_manifest_storer_data(value.manifests),
                                            class_id: class_id_challenge,
                                            asset_id: asset_id_challenge,
                                        })
                                        .await;
                                    }
                                };
                            }
                        }
    
                        // Generate a random challenge each cycle
                        if let Ok(generated_challenge) = generate_challenge(GenerateChallengeInput {
                            seed: seeded.seed.clone(),
                        })
                        .await
                        {
                            info!("STEP 2: GENERATED CHALLENGE {:#?}", generated_challenge);
                        } else {
                            info!("STEP 2: NO ACCOUNTS TO CHALLENGE");
                        }
    
                        // Get all the manifests from the network on-chain
                        let all_manifests = get_manifests(None, None, None).await;
    
                        // Verify that the Result is the expected value
                        if let Ok(current_all_manifests) = all_manifests {
                            // If there are no manifest in the network the calculation is skipped
                            if current_all_manifests.manifests.len() > 0 {
                                // Get the cummulative size of all the network manifets
                                let network_size = get_cumulative_size(&current_all_manifests).await as f64;
    
                                // Calculate the labor tokens corresponded for the user
                                let rewards = calculate_daily_rewards(config.clone(), network_size, &seeded).await;
    
                                info!("STEP 3: CALCULATE REWARDS:");
                                info!("  Mining Rewards: {:?}", rewards.daily_mining_rewards);
                                info!("  Storage Rewards: {:?}", rewards.daily_storage_rewards);
    
                                daily_rewards += rewards.daily_storage_rewards;
                                daily_rewards += rewards.daily_mining_rewards;
    
                                //MINT DAILY REWARDS
                                if let Some(_pool_id) = pool_id {
                                    let mint = mint_labor_tokens(MintLaborTokensInput {
                                        seed: seeded.seed.clone(),
                                        amount: Balance::try_from(daily_rewards as u128).unwrap(),
                                        class_id: class_id_labor,
                                        asset_id: asset_id_labor,
                                    })
                                    .await;
                                    info!("STEP 4: MINT LABOR TOKENS: {:#?}", mint);
                                } else {
                                    info!("STEP 4: ERROR WHEN TRIED TO MINT LABOR TOKENS: INVALID POOL_ID");
                                }
                            }
                        }
                        println!(
                            "DAY: {} CYCLE: {} REWARDS: {}",
                            cycle / config.cycles_advance as u64,
                            cycle,
                            daily_rewards
                        );
                        tokio::time::sleep(time::Duration::from_millis(config.time_between_cycles_miliseconds)).await;
                    }
                }
            }
        });
    });
}

async fn calculate_daily_rewards(
    config: Config,
    network_size: f64,
    seeded: &SeededAccountOutput,
) -> Rewards {
    let mut rewards = Rewards {
        daily_mining_rewards: 0.0,
        daily_storage_rewards: 0.0,
    };
    let pool_id = get_account_pool_id(seeded.account.clone()).await;

    if let Ok(storer_manifest_data) =
        get_manifests_storage_data(pool_id, Some(seeded.account.clone())).await
    {
        rewards = calculate_rewards(config, &storer_manifest_data, network_size).await;

        return rewards;
    } else {
        return rewards;
    }
}

pub async fn calculate_rewards(
    config: Config,
    manifests: &GetAllManifestsStorerDataOutput,
    network_size: f64,
) -> Rewards {
    let daily_tokens_mining: f64 = config.yearly_tokens as f64 * 0.70 / (12 * 30) as f64;
    let daily_tokens_storage: f64 = config.yearly_tokens as f64 * 0.20 / (12 * 30) as f64;

    let mut rewards = Rewards {
        daily_mining_rewards: 0.0,
        daily_storage_rewards: 0.0,
    };

    let client = ipfs_api::IpfsClient::default();

    for manifest in manifests.manifests.iter() {
        let mut file_participation = 0.0;

        if let Ok(_req) = client
            .pin_ls(Some(&String::from(&manifest.cid.clone())), None)
            .await
        {
            if let Ok(file_check) = client
                .block_stat(&String::from(&manifest.cid.clone()))
                .await
            {
                file_participation = file_check.size as f64 / network_size;
            }
            // When the active cycles reached {NUMBER_CYCLES_TO_ADVANCE} which is equal to 1 day, the manifest active days are increased and the rewards are calculated
            if manifest.active_cycles >= config.cycles_advance {
                let active_days = manifest.active_days + 1;

                // The calculation of the storage rewards
                rewards.daily_storage_rewards += (1 as f64
                    / (1 as f64 + (-0.1 * (active_days - 45) as f64).exp()))
                    * daily_tokens_storage
                    * file_participation;

                // The calculation of the mining rewards
                rewards.daily_mining_rewards += daily_tokens_mining as f64 * file_participation;
            }
        }
    }
    return rewards;
}

async fn convert_to_validator(
    input: ConvertToValidatorInput,
) -> Result<ConvertToValidatorOutput, RequestError> {
    let result: Result<ConvertToValidatorOutput, _> =
        fula_contract_req("convert_to_validator", input).await;
    return result;
}

async fn setup(
) -> Result<Config, RequestError> {
    let result: Result<Config, _> =
        fula_contract_req("setup", ()).await;
    return result;
}

async fn refund(
    input: RefundInput,
) -> Result<RefundOutput, RequestError> {
    let result: Result<RefundOutput, _> =
        fula_contract_req("refund", input).await;
    return result;
}

async fn validate_current_manifests(
    input: VerifyManifestsInput,
) -> Result<VerifyManifestsOutput, RequestError> {
    let result: Result<fula::VerifyManifestsOutput, _> =
        fula_sugarfunge_req("fula/manifest/verify", input).await;
    return result;
}

async fn generate_challenge(
    input: GenerateChallengeInput,
) -> Result<GenerateChallengeOutput, RequestError> {
    let result: Result<challenge::GenerateChallengeOutput, _> =
        fula_sugarfunge_req("fula/challenge/generate", input).await;
    return result;
}

async fn verify_pending_challenge(
    input: VerifyPendingChallengeInput,
) -> Result<VerifyPendingChallengeOutput, RequestError> {
    let result: Result<challenge::VerifyPendingChallengeOutput, _> =
        fula_sugarfunge_req("fula/challenge/pending", input).await;
    return result;
}

async fn verify_challenge(
    input: VerifyChallengeInput,
) -> Result<GenerateChallengeOutput, RequestError> {
    let result: Result<challenge::GenerateChallengeOutput, _> =
        fula_sugarfunge_req("fula/challenge/verify", input).await;
    return result;
}

async fn mint_labor_tokens(
    input: MintLaborTokensInput,
) -> Result<MintLaborTokensOutput, RequestError> {
    let result: Result<challenge::MintLaborTokensOutput, _> =
        fula_sugarfunge_req("fula/mint_labor_tokens", input).await;
    return result;
}

async fn verify_file_size(
    input: VerifyFileSizeInput,
) -> Result<VerifyFileSizeOutput, RequestError> {
    let result: Result<challenge::VerifyFileSizeOutput, _> =
        fula_sugarfunge_req("fula/file/verify", input).await;
    return result;
}

async fn provide_file_size(
    input: ProvideFileSizeInput,
) -> Result<ProvideFileSizeOutput, RequestError> {
    let result: Result<challenge::ProvideFileSizeOutput, _> =
        fula_sugarfunge_req("fula/file/provide", input).await;
    return result;
}
