use bytes::Bytes;

pub type BoxBody = http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>;
