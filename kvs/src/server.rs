use tokio::net::{TcpListener, TcpStream};
use std::future::Future;
use tracing::{debug, error, info, instrument};

use bytes::BytesMut;

use crate::{Connection, Command, Db};

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
        let db = Db::new();
        loop {
            let (socket, addr) = self.listener.accept().await?;
            info!("A Connection accepted from addr: {:?}", addr);

            let mut connection = Connection::new(socket);

            let cloned_db = db.clone();

            tokio::spawn(async move {
                let frame = match connection.read().await {
                    Ok(Some(frame)) => frame,
                    Ok(None) => {
                        debug!("Could not read");
                        return
                    },

                    Err(e) => {
                        error!(cause = ?e, "connection error");
                        return;
                    }
                };

                let cmd = match Command::from_frame(frame) {
                    Ok(cmd) => cmd,
                    Err(e) => {
                        error!(cause = ?e, "connection error");
                        return;
                    }
                };

                debug!("Recived {:?}", cmd);

                cmd.apply(&cloned_db);


            });

        }
    }
}



