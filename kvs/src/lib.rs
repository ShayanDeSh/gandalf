#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

mod connection;
pub use connection::Connection;

pub mod server;

pub mod frame;
pub use frame::Frame;

pub mod parse;
pub use parse::Parse;

pub mod command;
pub use command::Command;

pub mod db;
pub use db::Db;
pub use db::DbGuard;

pub mod shutdown;
pub use shutdown::Shutdown;

pub const DEFAULT_PORT: &str = "9736";
pub const MAX_CONNECTIONS: usize = 250;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub type Result<T> = std::result::Result<T, Error>;

