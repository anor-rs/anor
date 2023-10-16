use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

/// Packet Type
#[derive(Debug, Clone, Copy)]
pub enum PacketType {
    StrorageInfo = 1,
    StrorageItemBlob = 2,
    StrorageItemObjectBlob = 3,
}

impl From<u8> for PacketType {
    fn from(v: u8) -> Self {
        match v {
            1 => PacketType::StrorageInfo,
            2 => PacketType::StrorageItemBlob,
            3 => PacketType::StrorageItemObjectBlob,
            _ => panic!("Unmatched PacketType value {}", v),
        }
    }
}

/// Codec Type
#[derive(Debug, Clone, Copy)]
pub enum CodecType {
    /// [Bincode](https://github.com/bincode-org/bincode)
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

impl From<u8> for CodecType {
    fn from(v: u8) -> Self {
        match v {
            1 => CodecType::Bincode,
            2 => CodecType::ProtocolBuffers,
            3 => CodecType::FlatBuffers,
            4 => CodecType::MessagePack,
            5 => CodecType::CapnProto,
            _ => panic!("Unmatched CodecType value {}", v),
        }
    }
}

pub struct Packet {
    pub header: PacketHeader,
    pub data: Vec<u8>,
}

/// Packet Header
#[derive(Debug)]
pub struct PacketHeader {
    pub packet_type: PacketType,
    pub packet_version: u8,
    pub codec_type: CodecType,
    pub data_length: u64,
}

const PACKET_HEADER_SIZE: usize = 11;

impl PacketHeader {
    pub fn to_vec(&self) -> Vec<u8> {
        let mut header: Vec<u8> = Vec::with_capacity(PACKET_HEADER_SIZE);
        header.push(self.packet_type as u8);
        header.push(self.packet_version);
        header.push(self.codec_type as u8);
        header.extend_from_slice(&(self.data_length.to_be_bytes()));
        header
    }
}

/*
/// Packet Header Version 02
#[derive(Debug)]
pub struct PacketHeaderVersion02 {
    pub packet_type: u8,
    pub packet_version: u8,
    pub codec_type: u8,
    pub data_length_bytes: u8,
    pub data_length: [u8],
}
*/

/// builds a packet
pub fn build_packet(data: Vec<u8>, packet_type: PacketType, codec_type: CodecType) -> Packet {
    let header = build_packet_header(&data, packet_type, codec_type);
    Packet { header, data }
}

/// parses a binary array into packet structure
pub fn parse_packet(buf: Vec<u8>) -> Result<Packet, String> {
    // parse header
    let header = parse_packet_header(&buf)?;

    // convert the buf into data part
    let mut data = buf;
    data.drain(0..PACKET_HEADER_SIZE);

    Ok(Packet { header, data })
}

/// builds a packet header
pub fn build_packet_header(
    data: &[u8],
    packet_type: PacketType,
    codec_type: CodecType,
) -> PacketHeader {
    let packet_version = 1;
    PacketHeader {
        packet_type,
        packet_version,
        codec_type,
        data_length: data.len() as u64,
    }
}

/// parses packet header
pub fn parse_packet_header(buf: &[u8]) -> Result<PacketHeader, String> {
    let buf_len = buf.len();
    if buf_len < PACKET_HEADER_SIZE {
        return Err(format!(
            "Cannot parse packet header, invalid data size: {}",
            buf_len
        ));
    }

    let mut data_length_arr = [0_u8; 8];
    data_length_arr.copy_from_slice(&buf[3..11]);

    let header = PacketHeader {
        packet_type: buf[0].into(),
        packet_version: buf[1],
        codec_type: buf[2].into(),
        data_length: u64::from_be_bytes(data_length_arr),
    };

    Ok(header)
}

/// encode object into binary array `[u8]`
pub fn encode_to_binary<T: bincode::Encode>(obj: &T, codec_type: CodecType) -> Option<Vec<u8>> {
    match codec_type {
        CodecType::Bincode => {
            let bincode_config = bincode::config::standard();
            match bincode::encode_to_vec(obj, bincode_config) {
                Ok(arr) => Some(arr),
                Err(msg) => {
                    log::error!("Object to Binary encode error: {}", msg.to_string());
                    None
                }
            }
        }
        _ => {
            log::error!("Codec {:?} not supported yet", codec_type);
            None
        }
    }
}

/// decode object from binary array slice `[u8]``
pub fn decode_from_binary<T: bincode::Decode>(encoded: &[u8], codec_type: CodecType) -> Option<T> {
    match codec_type {
        CodecType::Bincode => {
            let bincode_config = bincode::config::standard();
            match bincode::decode_from_slice(encoded, bincode_config) {
                Ok(r) => {
                    let (decoded, _len): (T, usize) = r;
                    Some(decoded)
                }
                Err(msg) => {
                    log::error!("Binary to Object decode error: {}", msg.to_string());
                    None
                }
            }
        }
        _ => {
            log::error!("Codec {:?} not supported yet", codec_type);
            None
        }
    }
}

/// Encodes the object and persists in file
pub fn encode_to_file<T: bincode::Encode>(obj: &T, filepath: PathBuf) -> Result<(), String> {
    let codec_type = CodecType::Bincode;
    if let Some(buf) = encode_to_binary(obj, codec_type) {
        match File::create(&filepath) {
            Ok(mut file) => {
                // build packet
                let packet = build_packet(buf, PacketType::StrorageItemBlob, codec_type);

                // write packet header
                if let Err(err) = file.write_all(&packet.header.to_vec()) {
                    return Err(format!(
                        "Could not write into file: `{}`, Error Message: {}",
                        filepath.to_string_lossy(),
                        err
                    ));
                }

                // write packet data
                if let Err(err) = file.write_all(&packet.data) {
                    return Err(format!(
                        "Could not write into file: `{}`, Error Message: {}",
                        filepath.to_string_lossy(),
                        err
                    ));
                }
            }
            Err(err) => {
                return Err(format!(
                    "Could not create file: `{}`, Error Message: {}",
                    filepath.to_string_lossy(),
                    err
                ));
            }
        }
    } else {
        return Err("Could not encode object!".to_string());
    }
    Ok(())
}

/// Loads and decodes object from file
pub fn decode_from_file<T: bincode::Decode>(filepath: PathBuf) -> Result<T, String> {
    if let Ok(mut file) = File::open(&filepath) {
        let mut buf = vec![];
        match file.read_to_end(&mut buf) {
            Ok(_) => match parse_packet(buf) {
                Ok(packet) => {
                    if let Some(obj) = decode_from_binary(&packet.data, packet.header.codec_type) {
                        return Ok(obj);
                    }
                }
                Err(err) => {
                    return Err(err);
                }
            },
            Err(err) => {
                return Err(format!(
                    "Could not read file: `{}`, Error Message: {}",
                    filepath.to_string_lossy(),
                    err
                ));
            }
        }
    }
    Err(format!(
        "Could not open file: {}",
        filepath.to_string_lossy()
    ))
}
