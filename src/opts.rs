use bevy::prelude::*;
use std::ffi::OsStr;
use structopt::StructOpt;
// use ipfs_api_backend_hyper as ipfs_api;
use sugarfunge_api_types::primitives::{ClassId, AssetId};

#[derive(Debug, StructOpt)]
#[structopt(name = "fula-funge", about = "Fula Box proof engine.")]
pub struct Opt {
    /// SugarFunge operator
    #[structopt(parse(from_os_str))]
    pub operator: OptOperator,
    /// SugarFunge pool class_id
    #[structopt(long = "pool-id", parse(from_os_str))]
    pub class_id: OptClassId,
    /// SugarFunge pool asset_id
    #[structopt(long = "asset-id", parse(from_os_str))]
    pub asset_id: OptAssetId,
}

#[derive(Debug, Deref)]
pub struct OptIpfsPeer(String);

impl From<&OsStr> for OptIpfsPeer {
    fn from(os_str: &OsStr) -> Self {
        let os_str = os_str.to_str().unwrap();
        OptIpfsPeer(os_str.to_string())
    }
}

#[derive(Debug, Deref)]
pub struct OptOperator(pub sugarfunge_api_types::primitives::Seed);

impl From<&OsStr> for OptOperator {
    fn from(os_str: &OsStr) -> Self {
        let os_str = os_str.to_str().unwrap();
        let seed = sugarfunge_api_types::primitives::Seed::from(os_str.to_string());
        OptOperator(seed)
    }
}

#[derive(Debug, Deref)]
pub struct OptClassId(ClassId);

impl From<&OsStr> for OptClassId {
    fn from(os_str: &OsStr) -> Self {
        let os_str = os_str.to_str().unwrap();
        let class_id: u64 = os_str.parse().unwrap();
        let class_id = ClassId::from(class_id);
        OptClassId(class_id)
    }
}

#[derive(Debug, Deref)]
pub struct OptAssetId(AssetId);

impl From<&OsStr> for OptAssetId {
    fn from(os_str: &OsStr) -> Self {
        let os_str = os_str.to_str().unwrap();
        let asset_id: u64 = os_str.parse().unwrap();
        let asset_id = AssetId::from(asset_id);
        OptAssetId(asset_id)
    }
}

// SBP-M1 review: consider adding option for port