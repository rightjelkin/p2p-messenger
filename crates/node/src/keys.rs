use anyhow::{Result};
use libp2p::{PeerId};
use crate::config::keypair_from_secp256k1_hex;


pub(crate) fn peer_id_from_priv_hex(hex_priv: &str) -> Result<PeerId> {
    let kp = keypair_from_secp256k1_hex(hex_priv).unwrap();
    Ok(kp.public().to_peer_id())
}
