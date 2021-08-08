use gandolf_consensus::raft::Raft;

use tracing_subscriber;

#[tokio::main]
pub async fn main() -> Result<(), gandolf_consensus::Error> {
    tracing_subscriber::fmt::try_init()?;

    let mut raft = Raft::new();
    raft.run().await;

    Ok(())
}


