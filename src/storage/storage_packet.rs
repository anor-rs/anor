const STORAGE_PACKET_HEADER_SIZE: usize = 11;
const STORAGE_PACKET_VERSION: u8 = 1;

/// Strorage Packet Type
#[derive(Debug, Clone, Copy)]
pub enum StroragePacketType {
    StrorageInfo = 1,
    StrorageItemBlob = 2,
    StrorageItemObjectBlob = 3,
}

impl From<u8> for StroragePacketType {
    fn from(v: u8) -> Self {
        match v {
            1 => StroragePacketType::StrorageInfo,
            2 => StroragePacketType::StrorageItemBlob,
            3 => StroragePacketType::StrorageItemObjectBlob,
            _ => panic!("Unmatched StroragePacketType value {}", v),
        }
    }
}

/// Strorage Codec Type
#[derive(Debug, Default, Clone, Copy)]
pub enum StrorageCodecType {
    /// [Bincode](https://github.com/bincode-org/bincode)
    #[default]
    Bincode = 1,

    /// [Protocol Buffers](https://protobuf.dev/)
    ProtocolBuffers = 2,

    /// [FlatBuffers](https://github.com/google/flatbuffers)
    FlatBuffers = 3,

    /// [MessagePack](https://msgpack.org/)
    MessagePack = 4,

    /// [Cap'n Proto](https://capnproto.org/)
    CapnProto = 5,
}

impl From<u8> for StrorageCodecType {
    fn from(v: u8) -> Self {
        match v {
            1 => StrorageCodecType::Bincode,
            2 => StrorageCodecType::ProtocolBuffers,
            3 => StrorageCodecType::FlatBuffers,
            4 => StrorageCodecType::MessagePack,
            5 => StrorageCodecType::CapnProto,
            _ => panic!("Unmatched CodecType value {}", v),
        }
    }
}

/// Strorage Packet
pub struct StroragePacket {
    pub header: StroragePacketHeader,
    pub data: Vec<u8>,
}

/// Strorage Packet Header
#[derive(Debug)]
pub struct StroragePacketHeader {
    pub packet_length: u64,
    pub packet_type: StroragePacketType,
    pub packet_version: u8,
    pub codec_type: StrorageCodecType,
}

impl StroragePacketHeader {
    pub fn to_vec(&self) -> Vec<u8> {
        let mut header: Vec<u8> = Vec::with_capacity(STORAGE_PACKET_HEADER_SIZE);
        header.extend_from_slice(&(self.packet_length.to_be_bytes()));
        header.push(self.packet_type as u8);
        header.push(self.packet_version);
        header.push(self.codec_type as u8);
        header
    }
}

/// builds a storage packet
pub fn build_storage_packet(
    buf: Vec<u8>,
    packet_type: StroragePacketType,
    codec_type: StrorageCodecType,
) -> StroragePacket {
    let header = build_storage_packet_header(&buf, packet_type, codec_type);
    StroragePacket { header, data: buf }
}

/// parses a buffer into storage packet
pub fn parse_storage_packet(buf: Vec<u8>) -> Result<StroragePacket, String> {
    // parse header
    let header = parse_storage_packet_header(&buf)?;

    // convert the buf into data part
    let mut data = buf;
    data.drain(0..STORAGE_PACKET_HEADER_SIZE);

    Ok(StroragePacket { header, data })
}

/// builds a storage packet header
pub fn build_storage_packet_header(
    buf: &[u8],
    packet_type: StroragePacketType,
    codec_type: StrorageCodecType,
) -> StroragePacketHeader {
    StroragePacketHeader {
        packet_type,
        packet_version: STORAGE_PACKET_VERSION,
        codec_type,
        packet_length: (buf.len() + STORAGE_PACKET_HEADER_SIZE) as u64,
    }
}

/// parses storage packet header
pub fn parse_storage_packet_header(buf: &[u8]) -> Result<StroragePacketHeader, String> {
    let buf_len = buf.len();
    if buf_len < STORAGE_PACKET_HEADER_SIZE {
        return Err(format!(
            "Cannot parse packet header, invalid buffer size: {}",
            buf_len
        ));
    }

    let mut packet_length_arr = [0_u8; 8];
    packet_length_arr.copy_from_slice(&buf[0..8]);

    let packet_length = u64::from_be_bytes(packet_length_arr);
    if buf_len != (packet_length as usize) {
        return Err(format!(
            "Invalid buffer size, expected: {}, found: {}",
            packet_length, buf_len
        ));
    }

    let header = StroragePacketHeader {
        packet_length,
        packet_type: buf[8].into(),
        packet_version: buf[9],
        codec_type: buf[10].into(),
    };

    Ok(header)
}
