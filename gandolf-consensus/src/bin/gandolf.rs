use gandolf_consensus::{server, ConfigMap};
use gandolf_consensus::{DEFAULT_PORT, HEARTBEAT, TIMEOUT};

use tracing_subscriber;
use tokio::signal;

use structopt::StructOpt;

use gandolf_consensus::client::kvs::{KvsParser, KvsTracker}; 


#[tokio::main]
pub async fn main() -> Result<(), gandolf_consensus::Error> {
    tracing_subscriber::fmt::try_init()?;

    let cli = Cli::from_args();
    println!("{:?}", cli);


    let nodes = cli.nodes.ok_or("You must pass list of nodes")?;

    let config = ConfigMap::new(cli.host, cli.port, nodes, cli.heartbeat,
        cli.timeout)?;

    let address = "127.0.0.1:9736".parse()?;

    let tracker = KvsTracker::new(address);

    server::run(signal::ctrl_c(), config, KvsParser, tracker).await?;

    Ok(())
}

#[derive(StructOpt, Debug)]
#[structopt(name = "gandolf", version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"), about = "gandolf consensus system")]
struct Cli {
    #[structopt(name = "port", short = "-p", long = "--port", default_value = DEFAULT_PORT)]
    port: u16,

    #[structopt(name = "host", short = "-h" ,long = "host", default_value = "127.0.0.1")]
    host: String,

    #[structopt(name = "nodes", long = "--node")]
    nodes: Option<Vec<String>>,

    #[structopt(name = "heartbeat", long = "--heart", default_value = HEARTBEAT)]
    heartbeat: u64,

    #[structopt(name = "timeout", long = "--timeout", default_value = TIMEOUT)]
    timeout: u64,
}
