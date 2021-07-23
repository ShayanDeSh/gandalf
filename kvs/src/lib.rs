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

pub const DEFAULT_PORT: &str = "9736";

pub type Error = Box<dyn std::error::Error + Send + Sync>;

