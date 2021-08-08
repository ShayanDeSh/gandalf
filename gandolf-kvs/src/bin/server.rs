use gandolf_kvs::server;

use structopt::StructOpt;
use tokio::net::TcpListener;
use tokio::signal;

use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
pub async fn main() -> Result<(), gandolf_kvs::Error> {
    tracing_subscriber::fmt::try_init()?;

    let cli = Cli::from_args();
    let port: &str = cli.port.as_deref().unwrap_or(gandolf_kvs::DEFAULT_PORT);
    let listener = TcpListener::bind(&format!("127.0.0.1:{}", port)).await?;

    info!("Listening to {:?}", port);

    server::run(listener, signal::ctrl_c()).await;
    Ok(())
}


#[derive(StructOpt, Debug)]
#[structopt(name = "gandolf-kvs-server", version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"), about = "gandolf's key value store")]
struct Cli {
    #[structopt(name = "port", short = "-p", long = "--port")]
    port: Option<String>
}


