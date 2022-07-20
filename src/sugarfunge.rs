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
use sugarfunge_api_types::{primitives::*, *};

#[derive(Deref)]
pub struct Sender<T>(pub channel::Sender<T>);
#[derive(Deref)]
pub struct Receiver<T>(pub channel::Receiver<T>);

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestError {
    pub message: serde_json::Value,
    pub description: String,
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

pub fn launch(sugar_rx: Res<Receiver<ProofEngine>>, tokio_runtime: Res<TokioRuntime>) {
    let rt = tokio_runtime.runtime.clone();

    let sugar_rx: channel::Receiver<ProofEngine> = sugar_rx.clone();

    std::thread::spawn(move || {
        let client = ipfs_api::IpfsClient::default();

        // Spawn the root task
        rt.block_on(async move {
            let proof = sugar_rx.recv().unwrap();

            let cmd_opts = Opt::from_args();

            let class_id = *cmd_opts.class_id;

            let ipfs_seed = format!("//fula/dev/2/{}", &proof.peer_id);

            let asset_id = calculate_hash(&ipfs_seed);
            let asset_id = AssetId::from(asset_id);

            info!("{:?} {:?}", class_id, asset_id);

            let seeded: account::SeededAccountOutput = req(
                "account/seeded",
                account::SeededAccountInput {
                    seed: Seed::from(ipfs_seed),
                },
            )
            .await
            .unwrap();
            info!("{:?}", seeded.seed);
            info!("{:?}", seeded.account);

            let operator_account: account::SeededAccountOutput = req(
                "account/seeded",
                account::SeededAccountInput {
                    seed: cmd_opts.operator.clone(),
                },
            )
            .await
            .unwrap();
            info!("operator: {:?}", operator_account.seed);
            info!("operator: {:?}", operator_account.account);

            let account_exists: account::AccountExistsOutput = req(
                "account/exists",
                account::AccountExistsInput {
                    account: seeded.account.clone(),
                },
            )
            .await
            .unwrap();
            info!("{:?}", account_exists);

            if !account_exists.exists {
                warn!("invalid: {:?}", seeded.account);
                warn!("registering: {:?}", seeded.account);

                let fund: account::FundAccountOutput = req(
                    "account/fund",
                    account::FundAccountInput {
                        seed: cmd_opts.operator.clone(),
                        to: seeded.account.clone(),
                        amount: Balance::from(1000000000000000000),
                    },
                )
                .await
                .unwrap();
                if u128::from(fund.amount) > 0 {
                    warn!("registered: {:?}", seeded.account);
                } else {
                    error!("could not register account");
                }
            }

            let class_info: asset::ClassInfoOutput =
                req("asset/class_info", asset::ClassInfoInput { class_id })
                    .await
                    .unwrap();

            if class_info.info.is_none() {
                info!("creating: {:?}", class_id);
                let create_class: asset::CreateClassOutput = req(
                    "asset/create_class",
                    asset::CreateClassInput {
                        seed: cmd_opts.operator.clone(),
                        owner: operator_account.account.clone(),
                        class_id,
                        metadata: json!({"fula":{"desc": "Proof engine token"}}),
                    },
                )
                .await
                .unwrap();
                info!("created: {:#?}", create_class);
            }

            let asset_info: asset::AssetInfoOutput =
                req("asset/info", asset::AssetInfoInput { class_id, asset_id })
                    .await
                    .unwrap();

            if asset_info.info.is_none() {
                info!("creating: {:?} {:?}", class_id, asset_id);
                let create_asset: asset::CreateOutput = req(
                    "asset/create",
                    asset::CreateInput {
                        seed: operator_account.seed.clone(),
                        class_id,
                        asset_id,
                        metadata: json!({"ipfs":{"root_hash": "0"}}),
                    },
                )
                .await
                .unwrap();
                info!("created: {:#?}", create_asset);
            }

            loop {
                if let Ok(proof) = sugar_rx.try_recv() {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                    let job_from = operator_account.account.clone();
                    let job_to = seeded.account.clone();

                    let manifests: Result<fula::ManifestsOutput, _> = req(
                        "fula/manifest",
                        fula::ManifestsInput {
                            from: job_from,
                            to: job_to,
                            manifest: json!(""),
                        },
                    )
                    .await;
                    info!("{:#?}", manifests);

                    if let Ok(manifests) = manifests {
                        info!(
                            "manifests.manifests[0].manifest: {:#?}",
                            manifests.manifests[0].manifest
                        );

                        if let Ok(manifest) = serde_json::from_value::<crate::manifest::Manifest>(
                            manifests.manifests[0].manifest.clone(),
                        ) {
                            info!("validating storage: {}", manifest.job.uri);
                            if let Ok(file_check) = client.block_stat(&manifest.job.uri).await {
                                info!("✅:  {:#?}", file_check);
                            }

                            info!("mint for: {:#?}", proof);

                            let mint: asset::MintOutput = match req(
                                "asset/mint",
                                asset::MintInput {
                                    seed: operator_account.seed.clone(),
                                    class_id,
                                    asset_id,
                                    to: seeded.account.clone(),
                                    amount: Balance::from(proof.cumulative_size as u128),
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

                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                            let metadata = json!({"ipfs":{"root_hash": proof.hash}});

                            info!(
                                "updating_metadata: {:?} {:?} {:#?}",
                                class_id, asset_id, metadata
                            );
                            let update_metadata: Result<asset::UpdateMetadataOutput, _> = req(
                                "asset/update_metadata",
                                asset::UpdateMetadataInput {
                                    seed: operator_account.seed.clone(),
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
            }
        });
    });
}
