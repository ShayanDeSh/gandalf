use kvs::server;

use structopt::StructOpt;
use tokio::net::TcpListener;
use tokio::signal;

#[tokio::main]
pub async fn main() -> Result<(), kvs::Error> {
    let cli = Cli::from_args();
    let port: &str = cli.port.as_deref().unwrap_or(kvs::DEFAULT_PORT);
    let listener = TcpListener::bind(&format!("127.0.0.1:{}", port)).await?;
    println!("{:?}", port);
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


