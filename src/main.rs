use bevy::prelude::*;
use common::*;
use crossbeam::channel::bounded;
use simula_core::{
    force_graph::SimulationParameters,
    signal::{SignalController, SignalFunction, SignalGenerator},
};

mod common;
mod ipfs;
mod manifest;
mod opts;
#[cfg(not(feature = "headless"))]
mod render;
mod sugarfunge;

#[cfg(not(feature = "headless"))]
fn main() {
    // Create the global tokio runtime for all async needs
    let runtime = std::sync::Arc::new(tokio::runtime::Runtime::new().unwrap());

    let (ipfs_tx, ipfs_rx) = bounded::<ipfs::ProofEngine>(1);
    let (sugar_tx, sugar_rx) = bounded::<ipfs::ProofEngine>(1);

    App::new()
        .register_type::<ipfs::ProofEngine>()
        .register_type::<SignalGenerator>()
        .register_type::<SignalFunction>()
        .register_type::<SignalController<f32>>()
        .register_type::<SimulationParameters>()
        .insert_resource(WindowDescriptor {
            title: "[Fula Funge] Proof Engine".to_string(),
            width: 940.,
            height: 528.,
            ..Default::default()
        })
        .insert_resource(TokioRuntime { runtime })
        .insert_resource(ipfs::Sender(ipfs_tx))
        .insert_resource(ipfs::Receiver(ipfs_rx))
        .insert_resource(sugarfunge::Sender(sugar_tx))
        .insert_resource(sugarfunge::Receiver(sugar_rx))
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(ClearColor(Color::rgb(0.125, 0.12, 0.13)))
        .add_plugins(DefaultPlugins)
        .add_plugin(render::RenderPlugin)
        .add_startup_system(ipfs::launch)
        .add_system(ipfs::handler)
        .add_startup_system(sugarfunge::launch)
        .run();
}

#[cfg(feature = "headless")]
fn main() {
    use bevy::log::LogPlugin;

    let runtime = std::sync::Arc::new(tokio::runtime::Runtime::new().unwrap());

    let (ipfs_tx, ipfs_rx) = bounded::<ipfs::ProofEngine>(1);
    let (sugar_tx, sugar_rx) = bounded::<ipfs::ProofEngine>(1);

    App::new()
        .register_type::<ipfs::ProofEngine>()
        .register_type::<SignalGenerator>()
        .register_type::<SignalFunction>()
        .register_type::<SignalController<f32>>()
        .register_type::<SimulationParameters>()
        .insert_resource(TokioRuntime { runtime })
        .insert_resource(ipfs::Sender(ipfs_tx))
        .insert_resource(ipfs::Receiver(ipfs_rx))
        .insert_resource(sugarfunge::Sender(sugar_tx))
        .insert_resource(sugarfunge::Receiver(sugar_rx))
        .add_plugins(MinimalPlugins)
        .add_plugin(LogPlugin::default())
        .add_startup_system(ipfs::launch)
        .add_system(ipfs::handler)
        .add_startup_system(sugarfunge::launch)
        .run();
}
