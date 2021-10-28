use serde::{Serialize, Deserialize};
use gandalf_consensus::{server, ConfigMap};
use gandalf_consensus::{DEFAULT_PORT, HEARTBEAT, TIMEOUT};

use tracing_subscriber;
use tokio::signal;

use structopt::StructOpt;

use gandalf_consensus::client::kvs::{KvsParser, KvsTracker}; 

fn read_config(path: &str) -> serde_yaml::Result<Option<Cli>> {
    let config_file = std::fs::File::open(path).ok();
    if let Some(file) = config_file {
        let config = serde_yaml::from_reader(file)?;
        return Ok(Some(config));
    }
    Ok(None)
}


#[tokio::main]
pub async fn main() -> Result<(), gandalf_consensus::Error> {
    tracing_subscriber::fmt::try_init()?;

    let cli = Cli::from_args();

    let cli = if let Some(conf) = 
        read_config(&cli.config).map_err(|err| err.to_string())? {
        conf
    } else {
        cli
    };

    let nodes = cli.nodes.ok_or("You must pass list of nodes")?;

    let config = ConfigMap::new(cli.host, cli.port, nodes, cli.heartbeat,
        cli.timeout, cli.connection_host, cli.connection_port, cli.snapshot_offset)?;

    let address = format!("{}:{}", cli.client_host, cli.client_port).parse()?;

    let tracker = KvsTracker::new(address, cli.snapshot_path, cli.snapshot_offset);

    server::run(signal::ctrl_c(), config, KvsParser, tracker).await?;

    Ok(())
}

#[derive(StructOpt, Debug, Serialize, Deserialize)]
#[structopt(name = "gandalf", version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"), about = "gandalf consensus system")]
struct Cli {
    #[structopt(name = "port", short = "-p", long = "--port", default_value = DEFAULT_PORT)]
    port: u16,

    #[structopt(name = "host", short = "-h" ,long = "host", default_value = "127.0.0.1")]
    host: String,

    #[structopt(name = "nodes", long = "--node")]
    nodes: Option<Vec<String>>,

    #[structopt(name = "heartbeat", long = "--heart", default_value = HEARTBEAT)]
    heartbeat: u64,

    #[structopt(name = "snapshot_offset", long = "--offset", default_value = "10")]
    snapshot_offset: u64,

    #[structopt(name = "timeout", long = "--timeout", default_value = TIMEOUT)]
    timeout: u64,

    #[structopt(name = "client_port", long = "--client_port", default_value = "9736")]
    client_port: u16,

    #[structopt(name = "client_host", long = "--client_host", default_value = "127.0.0.1")]
    client_host: String,

    #[structopt(name = "connection_port", long = "--connection_port", default_value = "9876")]
    connection_port: u16,

    #[structopt(name = "connection_host", long = "--connection_host", default_value = "127.0.0.1")]
    connection_host: String,

    #[structopt(name = "snapshot_path", long = "--snap", default_value = "/tmp")]
    snapshot_path: String,

    #[structopt(name = "config", long = "--config", default_value = "/etc/gandalf.conf")]
    #[serde(skip)]
    config: String
}
