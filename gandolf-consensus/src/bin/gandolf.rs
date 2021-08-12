use gandolf_consensus::raft;

use tracing_subscriber;

use tokio::signal;


#[tokio::main]
pub async fn main() -> Result<(), gandolf_consensus::Error> {
    tracing_subscriber::fmt::try_init()?;

    let addr = "127.0.0.1:10000".parse().unwrap();

    raft::run(signal::ctrl_c(), addr).await;

    Ok(())
}


