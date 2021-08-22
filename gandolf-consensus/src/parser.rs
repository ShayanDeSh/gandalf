use bytes::BytesMut;

use crate::ClientData;

pub trait Parser<T: ClientData>: Send + Sync + Clone + 'static {
    fn parse(&self, buffer: &mut BytesMut) -> crate::Result<Option<T>>;
}
