use gandolf_consensus::raft;

use tracing_subscriber;

use tokio::signal;

#[tokio::main]
pub async fn main() -> Result<(), gandolf_consensus::Error> {
    tracing_subscriber::fmt::try_init()?;

    raft::run(signal::ctrl_c()).await;

    Ok(())
}


