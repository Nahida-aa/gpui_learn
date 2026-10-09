
impl Envelope {
    #[inline(never)]
    pub fn decode_from_slice(buffer: &[u8]) -> Result<Self, DecodeError> { Self::decode(buffer) }

    #[inline(never)]
    pub fn encode_to_buffer(&self, buffer: &mut Vec<u8>) -> Result<(), prost::EncodeError> {
        self.encode(buffer)
    }

    #[inline(never)]
    pub fn encoded_size(&self) -> usize { self.encoded_len() }
}
