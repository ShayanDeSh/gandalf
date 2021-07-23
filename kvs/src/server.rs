use tokio::net::TcpListener;
use std::future::Future;
use tracing::{debug, error, info, instrument};

use bytes::BytesMut;

use crate::{Connection};

pub async fn run(listener: TcpListener, shutdown: impl Future) {
    tokio::select! {
        res = listener.accept() => {
            let socket = match res {
                Ok((socket, _)) => socket,
                Err(e) => {
                    error!(cause = %e, "failed to accept");
                    return;
                }
            };
            info!("A Connection accepted");
            let mut connection = Connection::new(socket);
            connection.read().await;
        }
        _ = shutdown => {
            info!("Shutting down");
        }
    }
}
