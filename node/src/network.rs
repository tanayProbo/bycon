//! Global libp2p P2P network: Gossipsub block propagation, bootnode dialing, and mDNS discovery.

use futures::stream::StreamExt;
use libp2p::{
    gossipsub,
    mdns,
    noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp,
    yamux,
    Multiaddr,
    SwarmBuilder,
};
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc::UnboundedReceiver, Mutex};
use tracing::{info, warn};

use protocol::block::Block;
use crate::chain::Blockchain;
use crate::state::StateDb;
use crate::execution::ExecutionEngine;
use ethereum_types::U256;

const BLOCK_REWARD_WEI: u128 = 3_000_000_000_000_000_000;

#[derive(NetworkBehaviour)]
struct ByconnBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

pub async fn start_p2p(
    chain: Arc<Mutex<Blockchain>>,
    state_db: Arc<Mutex<StateDb>>,
    mut block_rx: UnboundedReceiver<Block>,
) -> Result<(), Box<dyn Error>> {
    info!("Initializing global libp2p swarm...");

    let id_keys = libp2p::identity::Keypair::generate_ed25519();
    let peer_id = libp2p::PeerId::from(id_keys.public());
    info!("Local Peer ID: {}", peer_id);

    let mut swarm = SwarmBuilder::with_existing_identity(id_keys)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|key| {
            let message_id_fn = |message: &gossipsub::Message| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                gossipsub::MessageId::from(s.finish().to_string())
            };

            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(1))
                .validation_mode(gossipsub::ValidationMode::Strict)
                .message_id_fn(message_id_fn)
                .build()
                .map_err(|msg| std::io::Error::new(std::io::ErrorKind::Other, msg))?;

            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )?;

            let mdns = mdns::tokio::Behaviour::new(
                mdns::Config::default(),
                key.public().to_peer_id(),
            )?;
            Ok(ByconnBehaviour { gossipsub, mdns })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    let topic = gossipsub::IdentTopic::new("byconn-blocks");
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    // Listen on deterministic P2P port (default 30303, override with BYCONN_P2P_PORT)
    let p2p_port = std::env::var("BYCONN_P2P_PORT").unwrap_or_else(|_| "30303".to_string());
    let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", p2p_port).parse()?;
    swarm.listen_on(listen_addr)?;

    // Dial any configured global seed/bootnodes (comma-separated Multiaddrs in BYCONN_BOOTNODES)
    if let Ok(bootnodes) = std::env::var("BYCONN_BOOTNODES") {
        for raw in bootnodes.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            match raw.parse::<Multiaddr>() {
                Ok(addr) => {
                    info!("Dialing global bootnode: {}", addr);
                    let _ = swarm.dial(addr);
                }
                Err(e) => warn!("Invalid bootnode multiaddr '{}': {}", raw, e),
            }
        }
    }

    info!("Global P2P network active on TCP port {}.", p2p_port);

    loop {
        tokio::select! {
            Some(mined_block) = block_rx.recv() => {
                let encoded = rlp::encode(&mined_block).to_vec();
                if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), encoded) {
                    warn!("Gossipsub block broadcast note: {:?}", e);
                } else {
                    info!("Broadcasted mined block #{} to global swarm", mined_block.header.number);
                }
            }
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(ByconnBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        info!("mDNS discovered peer: {peer_id}");
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                }
                SwarmEvent::Behaviour(ByconnBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    }
                }
                SwarmEvent::Behaviour(ByconnBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message,
                    ..
                })) => {
                    if let Ok(remote_block) = rlp::decode::<Block>(&message.data) {
                        let chain_guard = chain.lock().await;
                        let local_height = chain_guard.latest_height().unwrap_or(0);
                        if remote_block.header.number == local_height + 1 {
                            let state_guard = state_db.lock().await;
                            let mut engine = ExecutionEngine::new(&state_guard);
                            let mut cumulative_gas = 0;
                            for tx in &remote_block.transactions {
                                if let Ok(r) = engine.execute_transaction(tx, remote_block.header.proposer, cumulative_gas) {
                                    cumulative_gas += r.gas_used;
                                }
                            }
                            if let Ok(mut proposer_acc) = state_guard.get_account(&remote_block.header.proposer) {
                                proposer_acc.balance = proposer_acc.balance.saturating_add(U256::from(BLOCK_REWARD_WEI));
                                let _ = state_guard.put_account(&remote_block.header.proposer, &proposer_acc);
                            }
                            if chain_guard.append_block(&remote_block).is_ok() {
                                info!("Accepted global block #{} from peer {}", remote_block.header.number, peer_id);
                            }
                        }
                    }
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("P2P listening on {address}");
                }
                _ => {}
            }
        }
    }
}
