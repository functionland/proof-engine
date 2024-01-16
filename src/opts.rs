use bevy::prelude::*;
use std::ffi::OsStr;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "fula-funge", about = "Fula Box proof engine.")]
pub struct Opt {
    /// SugarFunge operator
    #[structopt(parse(from_os_str))]
    pub seed: SeedOperator,

    #[structopt(parse(from_os_str))]
    pub aura: AccountOperator,

    #[structopt(parse(from_os_str))]
    pub grandpa: AccountOperator,
}

#[derive(Debug, Deref)]
pub struct AccountOperator(String);

impl From<&OsStr> for AccountOperator {
    fn from(os_str: &OsStr) -> Self {
        let os_str = os_str.to_str().unwrap();
        AccountOperator(os_str.to_string())
    }
}

#[derive(Debug, Deref)]
pub struct SeedOperator(pub sugarfunge_api_types::primitives::Seed);

impl From<&OsStr> for SeedOperator {
    fn from(os_str: &OsStr) -> Self {
        let os_str = os_str.to_str().unwrap();
        let seed = sugarfunge_api_types::primitives::Seed::from(os_str.to_string());
        SeedOperator(seed)
    }
}
