use gandolf_kvs::server;

use structopt::StructOpt;
use tokio::net::TcpListener;
use tokio::signal;

use std::env;

use tracing::info;
use tracing_subscriber;

#[tokio::main]
pub async fn main() -> Result<(), gandolf_kvs::Error> {
    tracing_subscriber::fmt::try_init()?;

    let cli = Cli::from_args();

    let port = env::var("GANDOLF_KVS_PORT").unwrap_or(cli.port);
    let host = env::var("GANDOLF_KVS_HOST").unwrap_or(cli.host);

    let listener = TcpListener::bind(&format!("{}:{}", host, port)).await?;

    info!("Listening to {}:{}", host, port);

    server::run(listener, signal::ctrl_c()).await;
    Ok(())
}


#[derive(StructOpt, Debug)]
#[structopt(name = "gandolf-kvs-server", version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"), about = "gandolf's key value store")]
struct Cli {
    #[structopt(name = "port", short = "-p", long = "--port", default_value = "127.0.0.1")]
    port: String,

    #[structopt(name = "host", long = "--host", default_value = gandolf_kvs::DEFAULT_PORT)]
    host: String,
}


