use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Work {
    Storage,
    Compute,
}

#[derive(Serialize, Deserialize)]
pub enum Engine {
    IPFS,
}

#[derive(Serialize, Deserialize)]
pub struct Job {
    pub work: Work,
    pub uri: String,
    pub engine: Engine,
}

#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub job: Job,
}
