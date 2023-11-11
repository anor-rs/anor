use super::storage_packet::*;
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

/// encode object into binary array `[u8]`
pub fn encode_to_binary<T: bincode::Encode>(
    obj: &T,
    codec_type: StrorageCodecType,
) -> Option<Vec<u8>> {
    match codec_type {
        StrorageCodecType::Bincode => {
            let bincode_config = bincode::config::standard();
            match bincode::encode_to_vec(obj, bincode_config) {
                Ok(arr) => Some(arr),
                Err(msg) => {
                    tracing::error!("Object to Binary encode error: {}", msg.to_string());
                    None
                }
            }
        }
        _ => {
            tracing::error!("Codec {:?} not supported yet", codec_type);
            None
        }
    }
}

/// decode object from binary array slice `[u8]``
pub fn decode_from_binary<T: bincode::Decode>(
    encoded: &[u8],
    codec_type: StrorageCodecType,
) -> Option<T> {
    match codec_type {
        StrorageCodecType::Bincode => {
            let bincode_config = bincode::config::standard();
            match bincode::decode_from_slice(encoded, bincode_config) {
                Ok(r) => {
                    let (decoded, _len): (T, usize) = r;
                    Some(decoded)
                }
                Err(msg) => {
                    tracing::error!("Binary to Object decode error: {}", msg.to_string());
                    None
                }
            }
        }
        _ => {
            tracing::error!("Codec {:?} not supported yet", codec_type);
            None
        }
    }
}

/// Encodes the object and persists in file
pub fn encode_to_file<T: bincode::Encode>(
    filepath: PathBuf,
    obj: &T,
    packet_type: StroragePacketType,
) -> Result<(), String> {
    let codec_type = StrorageCodecType::default();
    if let Some(buf) = encode_to_binary(obj, codec_type) {
        match File::create(&filepath) {
            Ok(mut file) => {
                // build packet
                let packet = build_storage_packet(buf, packet_type, codec_type);

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
