use bytes::BytesMut;

pub trait Parser<T> {
    fn parse(&self, buffer: &mut BytesMut) -> crate::Result<Option<T>>;
}
