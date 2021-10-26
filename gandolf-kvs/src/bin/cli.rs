use gandolf_kvs::{client, DEFAULT_PORT};

use bytes::Bytes;
use std::str;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "gandolf-kvs", version = env!("CARGO_PKG_VERSION"), author = env!("CARGO_PKG_AUTHORS"), about = "Issue Redis commands")]
struct Cli {
    #[structopt(subcommand)]
    command: Command,

    #[structopt(name = "hostname", long = "--host", default_value = "127.0.0.1")]
    host: String,

    #[structopt(name = "port", long = "--port", default_value = DEFAULT_PORT)]
    port: String,
}


#[derive(StructOpt, Debug)]
enum Command {
    Get {
        key: String,
    },
    Set {
        key: String,

        #[structopt(parse(from_str = bytes_from_str))]
        value: Bytes,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> gandolf_kvs::Result<()> {
    tracing_subscriber::fmt::try_init()?;

    let cli = Cli::from_args();

    let addr = format!("{}:{}", cli.host, cli.port);

    let mut client = client::connect(&addr).await?;

    match cli.command {
        Command::Get { key } => {
            if let Some(value) = client.get(&key).await? {
                if let Ok(string) = str::from_utf8(&value) {
                    println!("\"{}\"", string);
                } else {
                    println!("{:?}", value);
                }
            } else {
                println!("(nil)");
            }
        }
        Command::Set {
            key,
            value
        } => {
            client.set(&key, value).await?;
            println!("OK");
        }
    }

    Ok(())
}

fn bytes_from_str(src: &str) -> Bytes {
    Bytes::from(src.to_string())
}
