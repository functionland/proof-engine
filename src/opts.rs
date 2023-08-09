use bevy::prelude::*;
use std::ffi::OsStr;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "fula-funge", about = "Fula Box proof engine.")]
pub struct Opt {
    /// SugarFunge operator
    #[structopt(parse(from_os_str))]
    pub operator: OptOperator,
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
