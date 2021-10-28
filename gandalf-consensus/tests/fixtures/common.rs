use gandalf_consensus::{Raft, ConfigMap, ClientData, Tracker};
use gandalf_consensus::server::Listener;
use gandalf_consensus::parser::Parser;
use gandalf_consensus::rpc::RaftRpcService;
use gandalf_consensus::raft_rpc::raft_rpc_server::RaftRpcServer;

use tokio::net::TcpListener;
use tokio::sync::{mpsc, RwLock};

use tonic::transport::Server;

use std::sync::Arc;
use std::net::SocketAddr;

use std::cell::RefCell;

pub async fn create_cluster<T: ClientData, R: Tracker<Entity=T>, P: Parser<T>>
(node_configs: Vec<NodeConfig>, tracker: Vec<R>, parser: P)
    -> gandalf_consensus::Result<Vec<(RefCell<Raft<T, R>>, SocketAddr)>> {
    let mut cluster = Vec::new();
    for (i, conf) in node_configs.into_iter().enumerate() {
        let kvs_addr = create_kvs_server().await;
        let raft = create_node(conf, tracker[i].clone(), parser.clone()).await?;
        cluster.append(&mut vec![(RefCell::new(raft), kvs_addr)]);
    }
    Ok(cluster)
}

pub async fn create_node<T: ClientData, R: Tracker<Entity=T>, P: Parser<T>>(conf: NodeConfig, tracker: R, parser: P) 
    -> gandalf_consensus::Result<Raft<T, R>> {
    let (tx_rpc, rx_rpc) = mpsc::unbounded_channel();

    let nodes = conf.nodes.ok_or("You must pass list of nodes")?;

    let config = ConfigMap::new(conf.host, conf.port, nodes, conf.heartbeat,
        conf.timeout, conf.connection_host, conf.connection_port, conf.snapshot_offset)?;

    let id = format!("{}:{}", config.host, config.port);
    let addr = format!("{}:{}", config.host, config.port).parse()?;

    let tcp_listener = TcpListener::bind(&format!("{}:{}",
            config.connecntion_host, config.connecntion_port)).await?;

    let mut listener = Listener::new(tcp_listener, tx_rpc.clone());

    tokio::spawn(async move {
            let _ = listener.run(parser).await;
        }
    );

    let raft_rpc = RaftRpcService::<T>::new(tx_rpc.clone());
    let svc = RaftRpcServer::new(raft_rpc);

    tokio::spawn(async move {
            let _ = Server::builder().add_service(svc).serve(addr).await;
        }
    );

    Ok(Raft::new(config, rx_rpc, Arc::new(RwLock::new(tracker)), id))
}

pub async fn create_kvs_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move { gandolf_kvs::server::run(listener, tokio::signal::ctrl_c()).await });

    addr
}


pub struct NodeConfig {
    pub port: u16,

    pub host: String,

    pub nodes: Option<Vec<String>>,

    pub heartbeat: u64,

    pub snapshot_offset: u64,

    pub timeout: u64,

    pub client_port: u16,

    pub client_host: String,

    pub connection_port: u16,

    pub connection_host: String,

    pub snapshot_path: String,
}
