use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::time::{self, Duration};

use std::future::Future;

use tokio::sync::{mpsc, oneshot};

use tonic::transport::Server;

use tracing::{info, error};

use crate::{Raft, ConfigMap, RaftMessage, ClientData, Tracker};

use crate::rpc::RaftRpcService;
use crate::raft_rpc::raft_rpc_server::RaftRpcServer;

use crate::parser::{Parser, Kind};

use bytes::BytesMut;

use std::marker::PhantomData;

pub struct Listener<P: Parser<T>, T: ClientData> {
    listener: TcpListener,
    tx_client: mpsc::UnboundedSender<RaftMessage<T>>,
    parser: PhantomData<P>
}

pub struct Handler<P: Parser<T>, T: ClientData> {
    stream: BufWriter<TcpStream>,
    tx_client: mpsc::UnboundedSender<RaftMessage<T>>,
    buffer: BytesMut,
    parser: P,
    data: PhantomData<T>
}


pub async fn run<T: ClientData, P: Parser<T>, R: Tracker<Entity=T>>(shutdown: impl Future,
    config: ConfigMap, parser: P, tracker: R) -> crate::Result<()> {
    let addr = format!("{}:{}", config.host, config.port).parse()?;
    let tcp_listener = TcpListener::bind(&format!("127.0.0.1:{}", 9999)).await?;

    let (tx_rpc, rx_rpc) = mpsc::unbounded_channel();

    let raft_rpc = RaftRpcService::<T>::new(tx_rpc.clone());

    let mut listener = Listener {
        listener: tcp_listener,
        tx_client: tx_rpc.clone(),
        parser: PhantomData
    };

    let svc = RaftRpcServer::new(raft_rpc);

    tokio::spawn(async move {
            let _ = Server::builder().add_service(svc).serve(addr).await;
        }
    );

    tokio::spawn(async move {
            let _ = listener.run(parser).await;
        }
    );

    let mut raft = Raft::new(config, rx_rpc, tracker);
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

impl<P: Parser<T>, T: ClientData> Listener<P, T> {
    pub async fn run(&mut self, parser: P) -> crate::Result<()> {
        loop {
            let socket = self.accept().await?;

            let mut handler = Handler {
                stream: BufWriter::new(socket),
                buffer: BytesMut::with_capacity(4096),
                tx_client: self.tx_client.clone(),
                parser: parser.clone(),
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

impl<P: Parser<T>, T: ClientData> Handler<P, T> {
    pub async fn run(&mut self) -> crate::Result<()> {
        loop {
            if let Some(frame) = self.parser.parse(&mut self.buffer)? {
                let (tx, rx) = oneshot::channel();
                let msg = match frame {
                    Kind::Read(frame) => RaftMessage::ClientReadMsg { body: frame, tx },
                    Kind::Write(frame) => RaftMessage::ClientWriteMsg { body: frame, tx }
                };
                self.tx_client.send(msg)?;

                let resp = rx.await?;

                match resp {
                    RaftMessage::ClientResp{body} =>  {
                        info!("{:?}", body);
                    },
                    _ => {return Err("Unkown response recived".into());}
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
