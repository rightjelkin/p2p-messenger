mod cli;
mod config;

use clap::Parser;
use tracing_subscriber::EnvFilter;
use libp2p::{swarm::SwarmEvent, noise, tcp, yamux, ping};
use std::time::Duration;
use futures::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let args = cli::Args::parse();
    let config = config::load_config(args.config_file);

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(config.keypair.clone())
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|_| ping::Behaviour::default())?
        .with_swarm_config(|cfg| {
            cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX))
        }) // Allows us to observe pings indefinitely.
        .build();

    swarm.listen_on(config.listen.clone())?;

    if let Some(addr) = config.bootnode {
        swarm.dial(addr.clone())?;
        println!("Dialed {addr}")
    }

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => println!("Listening on {address:?}"),
            SwarmEvent::Behaviour(event) => println!("{event:?}"),
            _ => {}
        }
    }
}
