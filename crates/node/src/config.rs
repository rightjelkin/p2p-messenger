use std::{path::PathBuf, error::Error};
use serde::Deserialize;
use libp2p::identity::{Keypair, secp256k1};
use hex::FromHex;
use libp2p::Multiaddr;

#[derive(Deserialize)]
pub struct RawAppConfig {
    pub private_key: String,
    pub listen: String,
    pub bootnodes: Option<Vec<String>>
}

pub struct AppConfig {
    pub keypair: Keypair,
    pub listen: Multiaddr,
    pub bootnodes: Option<Vec<Multiaddr>>
}

pub(crate) fn keypair_from_secp256k1_hex(hex_priv: &str) -> Result<Keypair, Box<dyn Error>> {
    let s = hex_priv.strip_prefix("0x").unwrap_or(hex_priv);

    let bytes = Vec::<u8>::from_hex(s)?;

    if bytes.len() != 32 {
        return Err(format!("expected 32 bytes private key, got {}", bytes.len()).into());
    }

    let mut bytes_mut = bytes;
    let secret = secp256k1::SecretKey::try_from_bytes(&mut bytes_mut)
        .map_err(|e| format!("failed to parse secp256k1 secret: {:?}", e))?;

    let secp_kp = secp256k1::Keypair::from(secret);

    Ok(Keypair::from(secp_kp))
}

pub fn load_config(config_path: PathBuf) -> AppConfig {

    let config_file_content = match std::fs::read_to_string(config_path) {
        Ok(file) => file,
        Err(e) => {
            println!("Error reading config file: {}", e);
            std::process::exit(1);
        },
    };

    let config: RawAppConfig = match toml::from_str(&config_file_content) {
        Ok(config) => config,
        Err(e) => {
            println!("Error parsing config file: {}", e);
            std::process::exit(1);
        },
    };

    AppConfig { 
        keypair: keypair_from_secp256k1_hex(&config.private_key).unwrap(),
        listen: config.listen.parse().unwrap(),
        bootnodes: config.bootnodes.map(|d| d.iter().map(|d| d.parse().unwrap()).collect())
    }
}