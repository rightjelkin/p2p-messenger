use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Envelope {
    pub msg_id: [u8; 32],
    pub from_pub: Vec<u8>,
    pub to_pub: Vec<u8>,
    pub payload: Vec<u8>,
}

pub(crate) fn compute_msg_id(env: &Envelope) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(&env.from_pub);
    h.update(&env.to_pub);
    h.update(&env.payload);
    let out = h.finalize();
    out.into()
}

pub enum Command {
    Submit { envelope: Envelope, reply: oneshot::Sender<ExecResult> },
    FetchLocal { recipient: Vec<u8>, since_ms: u64, limit: usize, reply: oneshot::Sender<Vec<Envelope>> },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecResult {
    pub msg_id: String,
    pub status: &'static str,
    pub replicas: usize,
}