use tokio::net::{TcpListener, TcpStream};
use std::future::Future;
use tracing::{debug, error, info, instrument};

use bytes::BytesMut;

use crate::{Connection};

pub struct Listener {
    listener: TcpListener
}


pub async fn run(listener: TcpListener, shutdown: impl Future) {
    let mut listener = Listener {
        listener: listener,
    };
    tokio::select! {
        res = listener.run() => {
            if let Err(err) = res {
                error!(cause = %err, "failed to accept");
            }
        }
        _ = shutdown => {
            info!("Shutting down");
        }
    }
}

impl Listener {
    pub async fn run(&mut self) -> crate::Result<()> {
        loop {
            let (socket, addr) = self.listener.accept().await?;
            info!("A Connection accepted from addr: {:?}", addr);

            let mut connection = Connection::new(socket);

            tokio::spawn(async move {
                let frame = match connection.read().await {
                    Ok(Some(frame)) => frame,
                    Ok(None) => return,
                    Err(e) => {
                        error!(cause = ?e, "connection error");
                        return;
                    }
                };

            });

        }
    }
}





