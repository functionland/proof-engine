use crate::{common::TokioRuntime, sugarfunge};
use bevy::prelude::*;
use crossbeam::channel;
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
    pub cumulative_size: u64,
    pub blocks: u64,
    pub total_in: u64,
    pub total_out: u64,
    pub rate_in: f64,
    pub rate_out: f64,
}

pub fn handler(mut ipfs_stats: Query<&mut ProofEngine>, ipfs_rx: Res<Receiver<ProofEngine>>) {
    if let Ok(msg) = channel::Receiver::try_recv(&ipfs_rx) {
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
            let peer = client.id(None).await.unwrap();
            info!("peer: {}", &peer.id);

            let _res = client
                .log_level(Logger::All, LoggingLevel::Error)
                .await
                .unwrap();

            let repo_stat = client.stats_repo().await.unwrap();
            info!("repo objs: {}", repo_stat.num_objects);
            info!("repo size: {}", repo_stat.repo_size);
            info!("repo path: {}", repo_stat.repo_path);
            info!("repo vers: {}", repo_stat.version);

            let mut root_hash = String::from("0");
            let mut hash_changed = false;

            loop {
                let root_stat = client.files_stat("/").await.unwrap();
                // debug!("{:#?}", root_stat);

                if root_hash != root_stat.hash {
                    root_hash = root_stat.hash.clone();
                    hash_changed = true;
                    info!("root   hash: {}", root_stat.hash);
                    info!("root   size: {}", root_stat.size);
                    info!("root blocks: {}", root_stat.blocks);
                }

                let bitswap_stats = client.stats_bitswap().await.unwrap();
                debug!("blocks recv: {}", bitswap_stats.blocks_received);
                debug!("data   recv: {}", bitswap_stats.data_received);
                debug!("blocks sent: {}", bitswap_stats.blocks_sent);
                debug!("data   sent: {}", bitswap_stats.data_sent);

                let bw_stat = client.stats_bw().await.unwrap();
                debug!("bw  total_in: {}", bw_stat.total_in);
                debug!("bw total_out: {}", bw_stat.total_out);
                debug!("bw   rate_in: {}", bw_stat.rate_in);
                debug!("bw  rate_out: {}", bw_stat.rate_out);

                let ledger = client.bitswap_ledger(&peer.id).await.unwrap();
                debug!("ledger     value: {}", ledger.value);
                debug!("ledger      sent: {}", ledger.sent);
                debug!("ledger      recv: {}", ledger.recv);
                debug!("ledger exchanged: {}", ledger.exchanged);

                println!(".");

                let proof = ProofEngine {
                    peer_id: peer.id.clone(),
                    hash: root_stat.hash.clone(),
                    size: root_stat.size,
                    cumulative_size: root_stat.cumulative_size,
                    blocks: root_stat.blocks,
                    total_in: bw_stat.total_in,
                    total_out: bw_stat.total_out,
                    rate_in: bw_stat.rate_in,
                    rate_out: bw_stat.rate_out,
                };

                ipfs_tx.send(proof.clone()).unwrap();

                if hash_changed {
                    sugar_tx.send(proof.clone()).unwrap();
                }

                hash_changed = false;

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
