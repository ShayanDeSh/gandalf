use tokio::net::{TcpListener, TcpStream};
use tokio::time::{self, Duration};
use tokio::sync::{Semaphore};
use tracing::{debug, error, info, instrument};

use std::future::Future;
use std::sync::Arc;

use crate::{Connection, Command, Db, DbGuard, MAX_CONNECTIONS};

pub struct Listener {
    listener: TcpListener,
    db_guard: DbGuard,
    connection_limit: Arc<Semaphore>,
}

pub struct Handler {
    db: Db,
    connection: Connection,
    connection_limit: Arc<Semaphore>,
}


pub async fn run(listener: TcpListener, shutdown: impl Future) {
    let mut listener = Listener {
        listener: listener,
        db_guard: DbGuard::new(),
        connection_limit: Arc::new(Semaphore::new(MAX_CONNECTIONS))
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
            self.connection_limit.acquire().await.unwrap().forget();
            
            let socket = self.accept().await?;

            let mut handler = Handler {
                connection: Connection::new(socket),
                db: self.db_guard.db(),
                connection_limit: self.connection_limit.clone()
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
        let maybe_frame = self.connection.read().await?;
        let frame = match maybe_frame {
            Some(frame) => frame,
            None => return Ok(())
        };

        let cmd = Command::from_frame(frame)?;

        debug!(?cmd);

        cmd.apply(&self.db, &mut self.connection).await?;

        Ok(())
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        self.connection_limit.add_permits(1);
    }
}

