use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::time::{self, Duration};

use std::future::Future;

use tokio::sync::mpsc;

use tonic::transport::Server;

use tracing::{info, error};

use crate::{Raft, ConfigMap, RaftMessage};

use crate::rpc::RaftRpcService;
use crate::raft_rpc::raft_rpc_server::RaftRpcServer;

use crate::parser::{Parser, ParsedData};

use bytes::BytesMut;

use crate::client::kvs::KvsParser;
use crate::client::kvs::parser::Frame;

use std::marker::PhantomData;

pub struct Listener {
    listener: TcpListener,
    tx_client: mpsc::UnboundedSender<RaftMessage>,
}

pub struct Handler<P: Parser<T>, T> {
    stream: BufWriter<TcpStream>,
    tx_client: mpsc::UnboundedSender<RaftMessage>,
    buffer: BytesMut,
    parser: P,
    data: PhantomData<T>
}


pub async fn run(shutdown: impl Future, config: ConfigMap) -> crate::Result<()> {
    let addr = format!("{}:{}", config.host, config.port).parse()?;
    let tcp_listener = TcpListener::bind(&format!("127.0.0.1:{}", 9999)).await?;

    let (tx_rpc, rx_rpc) = mpsc::unbounded_channel();

    let raft_rpc = RaftRpcService::new(tx_rpc.clone());

    let mut listener = Listener {
        listener: tcp_listener,
        tx_client: tx_rpc.clone()
    };

    let svc = RaftRpcServer::new(raft_rpc);

    tokio::spawn(async move {
            let _ = Server::builder().add_service(svc).serve(addr).await;
        }
    );

    tokio::spawn(async move {
            let _ = listener.run().await;
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

impl Listener {
    pub async fn run(&mut self) -> crate::Result<()> {
        loop {
            let socket = self.accept().await?;

            let mut handler = Handler {
                stream: BufWriter::new(socket),
                buffer: BytesMut::with_capacity(4096),
                tx_client: self.tx_client.clone(),
                parser: KvsParser,
                data: PhantomData
            };

            tokio::spawn(async move {
                    let _ = handler.run().await;
                }
            );

        }
    }

    pub async fn accept(&mut self) -> crate::Result<TcpStream> {
        let mut backoff = 1;

        loop {
            match self.listener.accept().await {
                Ok((socket, addr)) => {
                    info!("A Connection accepted from addr: {:?}", addr);
                    return Ok(socket)
                }
                Err(err) => {
                    if backoff > 64 {
                        return Err(err.into());
                    }
                }
            }
            time::sleep(Duration::from_secs(backoff)).await;

            backoff *= 2;
        }
    }
}

impl Handler<KvsParser, Frame> {
    pub async fn run(&mut self) -> crate::Result<()> {
        loop {
            if let Some(frame) = self.parser.parse(&mut self.buffer)? {
                match frame {
                        Frame::Integer(a) => info!(a),
                        _ => info!("fa")
                }
            }

            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                if self.buffer.is_empty() {
                    return Ok(());
                } else {
                    return Err("connection reset by peer".into());
                }
            }

        }
    }
}
