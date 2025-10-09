mod cli;
mod config;
mod keys;

use clap::Parser;
use tracing_subscriber::EnvFilter;
use libp2p::{autonat, identify, kad, noise, ping, swarm::{NetworkBehaviour, SwarmEvent}, tcp, yamux, StreamProtocol, PeerId, multiaddr::Protocol};
use std::time::Duration;
use futures::prelude::*;
use api::listen_api;

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    ping: ping::Behaviour,
    identify: identify::Behaviour,
    autonat: autonat::Behaviour,
    kademlia: kad::Behaviour<kad::store::MemoryStore>,
}

fn extract_peer_id(addr: &libp2p::Multiaddr) -> Option<PeerId> {
    use libp2p::multiaddr::Protocol;
    addr.iter().find_map(|p| match p {
        Protocol::P2p(mh) => PeerId::from_multihash(mh.into()).ok(),
        _ => None
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let args = cli::Args::parse();

    if let Some(cli::Command::PeerId { privkey }) = &args.command {
        let peer = keys::peer_id_from_priv_hex(privkey)?;
        println!("{peer}");
        return Ok(());
    }

    let config = config::load_config(args.config_file.expect("config file is required"));

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(config.keypair.clone())
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_behaviour(|key| {
            let local_peer_id = key.public().to_peer_id();
            let identify = identify::Behaviour::new(
                identify::Config::new("/p2p-mes/1.0.0".into(), key.public())
            );
            let ping = ping::Behaviour::default();
            let autonat = autonat::Behaviour::new(local_peer_id, Default::default());
            let mut kad_cfg = kad::Config::new(StreamProtocol::new("/p2p-mes/kad/1.0.0"));
            kad_cfg.set_query_timeout(Duration::from_secs(60));
            let store = kad::store::MemoryStore::new(local_peer_id);
            let kademlia = kad::Behaviour::with_config(local_peer_id, store, kad_cfg);
        
            Ok(MyBehaviour { ping, identify, autonat, kademlia })
        })?
        .with_swarm_config(|cfg| {
            cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX))
        }) // Allows us to observe pings indefinitely.
        .build();

    swarm.listen_on(config.listen.clone())?;

    for addr in &config.bootnodes.unwrap_or(vec![]) {
        if let Some(p) = extract_peer_id(addr) {
            swarm.behaviour_mut().kademlia.add_address(&p, addr.clone());
        }
        let _ = swarm.dial(addr.clone());
    }
    let local_peer_id = swarm.local_peer_id().clone();
    let _ = swarm.behaviour_mut().kademlia.get_closest_peers(local_peer_id);

    let http_bind: std::net::SocketAddr = config.listen_api.clone().unwrap();
    tokio::spawn(listen_api(http_bind));

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                let mut a = address.clone();
                let peer_id = swarm.local_peer_id().clone();
                let mh = peer_id.into();
                a.push(Protocol::P2p(mh));
                println!("Listening (share): {a}");
            },
            SwarmEvent::Behaviour(MyBehaviourEvent::Kademlia(ev)) => {
                use kad::{Event as KEvent, QueryResult};
    
                match ev {
                    KEvent::OutboundQueryProgressed { result, .. } => match result {
                        QueryResult::GetClosestPeers(Ok(ok)) => {
                            if ok.peers.is_empty() {
                                println!("KAD: closest peers: none (network small / not connected yet)");
                            } else {
                                println!("KAD: closest peers: {:?}", ok.peers);
                            }
                        }
                        QueryResult::GetClosestPeers(Err(e)) => {
                            println!("KAD: get_closest_peers error: {e:?}");
                        }
                        QueryResult::Bootstrap(Ok(_ok)) => {
                            println!("KAD: bootstrap OK");
                        }
                        QueryResult::Bootstrap(Err(e)) => {
                            println!("KAD: bootstrap error: {e:?}");
                        }
                        _ => {}
                    },
                    KEvent::RoutingUpdated { peer, is_new_peer, addresses, .. } => {
                        if is_new_peer {
                            println!("KAD: learned peer {peer} addrs={addresses:?}");
                        }
                    }
                    _ => {}
                }
            }
            SwarmEvent::Behaviour(MyBehaviourEvent::Autonat(
                autonat::Event::StatusChanged { old, new }
            )) => println!("AutoNAT: {old:?} -> {new:?}"),
            SwarmEvent::Behaviour(other) => println!("{other:?}"),
            _ => {}
        }
    }
}
