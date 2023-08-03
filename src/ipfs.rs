use crate::{common::TokioRuntime, sugarfunge};
use bevy::prelude::*;
use crossbeam::channel;
// SBP-M1 review: nest use statements
use ipfs_api::IpfsApi;
use ipfs_api::{Logger, LoggingLevel};
pub use ipfs_api_backend_hyper as ipfs_api;

#[derive(Deref)]
pub struct Sender<T>(pub channel::Sender<T>);
#[derive(Deref)]
pub struct Receiver<T>(pub channel::Receiver<T>);

#[derive(Component)]
struct DAG;

#[derive(Default, Debug, Component, Clone, Reflect)]
#[reflect(Component)]

pub struct ProofEngine {
    pub peer_id: String,
    pub hash: String,
    pub size: u64,
    pub cumulative_size: u64, //Storage provided by user
    pub blocks: u64,
    pub total_in: u64,
    pub total_out: u64,
    pub rate_in: f64,
    pub rate_out: f64,
}

pub fn handler(mut ipfs_stats: Query<&mut ProofEngine>, ipfs_rx: Res<Receiver<ProofEngine>>) {
    if let Ok(msg) = channel::Receiver::try_recv(&ipfs_rx) {
        // SBP-M1 review: remove commented out code
        // println!("ipfs_stat: {:#?}", msg);
        for mut ipfs_stat in ipfs_stats.iter_mut() {
            *ipfs_stat = msg.clone();
        }
    }
}

pub fn launch(
    mut commands: Commands,
    ipfs_tx: Res<Sender<ProofEngine>>,
    sugar_tx: Res<sugarfunge::Sender<ProofEngine>>,
    tokio_runtime: Res<TokioRuntime>,
) {
    let ipfs_tx: channel::Sender<ProofEngine> = ipfs_tx.clone();
    let sugar_tx: channel::Sender<ProofEngine> = sugar_tx.clone();

    // Use the tokio runtime from resources
    let rt = tokio_runtime.runtime.clone();

    std::thread::spawn(move || {
        let client = ipfs_api::IpfsClient::default();

        // Spawn the root task
        rt.block_on(async move {
            let mut root_hash = String::from("0");

            loop {
                let peer = match client.id(None).await {
                    Ok(peer) => peer,
                    Err(err) => {
                        error!("peer: {:#?}", err);
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };

                let _res = match client.log_level(Logger::All, LoggingLevel::Error).await {
                    Ok(log) => log,
                    Err(err) => {
                        error!("logger setup: {:#?}", err);
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };

                let repo_stat = match client.stats_repo().await {
                    Ok(repo_stat) => repo_stat,
                    Err(err) => {
                        error!("repo_stat: {:#?}", err);
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };

                debug!("repo objs: {}", repo_stat.num_objects);
                debug!("repo size: {}", repo_stat.repo_size);
                debug!("repo path: {}", repo_stat.repo_path);
                debug!("repo vers: {}", repo_stat.version);

                let root_stat = match client.files_stat("/").await {
                    Ok(file_stat) => file_stat,
                    Err(err) => {
                        error!("root_stat: {:#?}", err);
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };

                // SBP-M1 review: remove commented out code
                // debug!("{:#?}", root_stat);

                if root_hash != root_stat.hash {
                    root_hash = root_stat.hash.clone();
                    debug!("root   hash: {}", root_stat.hash);
                    debug!("root   size: {}", root_stat.size);
                    debug!("root blocks: {}", root_stat.blocks);
                }

                let bitswap_stats = match client.stats_bitswap().await {
                    Ok(bitswap_stat) => bitswap_stat,
                    Err(err) => {
                        error!("bitswap_stats: {:#?}", err);
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };
                debug!("blocks recv: {}", bitswap_stats.blocks_received);
                debug!("data   recv: {}", bitswap_stats.data_received);
                debug!("blocks sent: {}", bitswap_stats.blocks_sent);
                debug!("data   sent: {}", bitswap_stats.data_sent);

                let bw_stat = match client.stats_bw().await {
                    Ok(bw_stat) => bw_stat,
                    Err(err) => {
                        error!("bw_stat: {}", err);
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };
                debug!("bw  total_in: {}", bw_stat.total_in);
                debug!("bw total_out: {}", bw_stat.total_out);
                debug!("bw   rate_in: {}", bw_stat.rate_in);
                debug!("bw  rate_out: {}", bw_stat.rate_out);

                let ledger = match client.bitswap_ledger(&peer.id).await {
                    Ok(ledger) => ledger,
                    Err(err) => {
                        error!("ledger: {:#?}", err);
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };
                debug!("ledger     value: {}", ledger.value);
                debug!("ledger      sent: {}", ledger.sent);
                debug!("ledger      recv: {}", ledger.recv);
                debug!("ledger exchanged: {}", ledger.exchanged);

                println!(".");

                let proof = ProofEngine {
                    peer_id: peer.id.clone(),
                    hash: root_stat.hash.clone(),
                    size: root_stat.size,
                    cumulative_size: sugarfunge::get_cumulative_size_proof(peer.id.clone()).await, //Storage provided by user
                    blocks: sugarfunge::get_blocks_proof(peer.id.clone()).await,
                    total_in: bw_stat.total_in,
                    total_out: bw_stat.total_out,
                    rate_in: bw_stat.rate_in,
                    rate_out: bw_stat.rate_out,
                };

                ipfs_tx.send(proof.clone()).unwrap();
                sugar_tx.send(proof.clone()).unwrap();

                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
    });

    commands
        .spawn()
        .insert(DAG {})
        .insert(ProofEngine::default())
        .insert(Name::new("Fula: DAG"));
}
