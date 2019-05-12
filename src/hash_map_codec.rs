use bytes::{BufMut, BytesMut};
use bytevec::{ByteDecodable, ByteEncodable};
use std::{collections::HashMap, io};
use tokio_codec::{Decoder, Encoder};

pub struct HashMapCodec;

impl Decoder for HashMapCodec {
    type Item = HashMap<String, f64>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<HashMap<String, f64>>, io::Error> {
        let decoded_map = <HashMap<String, f64>>::decode::<u32>(&buf).expect("unable to decode");

        Ok(Some(decoded_map))
    }
}

impl Encoder for HashMapCodec {
    type Item = HashMap<String, f64>;
    type Error = io::Error;

    fn encode(
        &mut self,
        hash_map: HashMap<String, f64>,
        buf: &mut BytesMut,
    ) -> Result<(), io::Error> {
        let bytes = hash_map.encode::<u32>().expect("unable to encode");

        buf.reserve(bytes.len());
        buf.put(bytes);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut codec = HashMapCodec {};

        let mut original_map: HashMap<String, f64> = HashMap::new();
        original_map.insert("x".to_string(), 1.0);

        let mut buf = BytesMut::new();
        codec.encode(original_map.clone(), &mut buf).unwrap();

        let decoded_map = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(original_map, decoded_map);
    }
}
