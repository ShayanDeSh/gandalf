use bytes::BytesMut;

pub trait Parser<T>: Send + Sync + Clone + 'static {
    fn parse(&self, buffer: &mut BytesMut) -> crate::Result<Option<T>>;
}
