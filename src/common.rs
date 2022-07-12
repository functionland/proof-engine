use bevy::prelude::*;
use tokio::runtime::Runtime;

#[derive(Component)]
pub struct TokioRuntime {
    pub runtime: std::sync::Arc<Runtime>,
}
