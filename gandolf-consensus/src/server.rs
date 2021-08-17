use std::future::Future;

use tokio::sync::mpsc;

use tonic::transport::Server;

use tracing::{info, error};

use crate::{Raft, ConfigMap};

use crate::rpc::RaftRpcService;
use crate::raft_rpc::raft_rpc_server::RaftRpcServer;

pub async fn run(shutdown: impl Future, config: ConfigMap) -> crate::Result<()> {
    let addr = format!("{}:{}", config.host, config.port).parse()?;

    let (tx_rpc, rx_rpc) = mpsc::unbounded_channel();

    let raft_rpc = RaftRpcService::new(tx_rpc);

    let svc = RaftRpcServer::new(raft_rpc);

    tokio::spawn(async move {
            let _ = Server::builder().add_service(svc).serve(addr).await;
        }
    );

    let mut raft = Raft::new(config, rx_rpc);
    tokio::select! {
        res = raft.run() => {
            if let Err(err) = res {
                error!(cause = %err, "Caused an error: ");
            }
        }
        _ = shutdown => {
            info!("Shutting down the server");
        }
    }
    Ok(())
}
