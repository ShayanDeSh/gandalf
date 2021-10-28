use tokio::net::{TcpListener, TcpStream};
use tokio::time::{self, Duration};
use tokio::sync::{Semaphore, mpsc, broadcast};
use tracing::{debug, error, info};

use std::future::Future;
use std::sync::Arc;

use crate::{Connection, Command, Db, DbGuard, MAX_CONNECTIONS, Shutdown};

pub struct Listener {
    listener: TcpListener,
    db_guard: DbGuard,
    connection_limit: Arc<Semaphore>,
    complete_rx: mpsc::Receiver<()>,
    complete_tx: mpsc::Sender<()>,
    shutdown_signal: broadcast::Sender<()> 
}

pub struct Handler {
    db: Db,
    connection: Connection,
    connection_limit: Arc<Semaphore>,
    _complete_tx: mpsc::Sender<()>,
    shutdown: Shutdown
}


pub async fn run(listener: TcpListener, shutdown: impl Future) {
    let (complete_tx, complete_rx) = mpsc::channel(1);
    let (shutdown_signal, _) = broadcast::channel(1); 

    let mut listener = Listener {
        listener: listener,
        db_guard: DbGuard::new(),
        connection_limit: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        complete_tx: complete_tx,
        complete_rx: complete_rx,
        shutdown_signal: shutdown_signal,
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

    let Listener {
        mut complete_rx,
        complete_tx,
        shutdown_signal,
        ..
    } = listener;

    drop(shutdown_signal);
    drop(complete_tx);

    complete_rx.recv().await;
}

impl Listener {
    pub async fn run(&mut self) -> crate::Result<()> {
        loop {
            self.connection_limit.acquire().await.unwrap().forget();
            
            let socket = self.accept().await?;


            let mut handler = Handler {
                connection: Connection::new(socket),
                db: self.db_guard.db(),
                connection_limit: self.connection_limit.clone(),
                _complete_tx: self.complete_tx.clone(),
                shutdown: Shutdown::new(self.shutdown_signal.subscribe())
            };

            tokio::spawn(async move {
                if let Err(err) = handler.run().await {
                    error!(cause = ?err, "connection error");
                }
            });
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


impl Handler {
    pub async fn run(&mut self) -> crate::Result<()> {
        while !self.shutdown.is_shutdown() {
            let maybe_frame = tokio::select! {
                res = self.connection.read() => res?,
                _   = self.shutdown.recv() => return Ok(())
            };

            let frame = match maybe_frame {
                Some(frame) => frame,
                None => return Ok(())
            };

            let cmd = Command::from_frame(frame)?;

            debug!(?cmd);

            cmd.apply(&self.db, &mut self.connection).await?;

        }
        Ok(())
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        self.connection_limit.add_permits(1);
    }
}

