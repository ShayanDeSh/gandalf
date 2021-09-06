use bytes::{Bytes, BytesMut};

use crate::ClientData;

pub enum Kind<T: ClientData> {
    Read(T),
    Write(T)
}

pub trait Parser<T: ClientData>: Send + Sync + Clone + 'static {
    fn parse(&self, buffer: &mut BytesMut) -> crate::Result<Option<Kind<T>>>;

    fn unparse(&self, data: T) -> crate::Result<Bytes>;
    
}
