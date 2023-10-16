/// Generic Type
#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub enum StorageType {
    Basic(BasicType),
    Complex(ComplexType),
}

/// Basic Type
#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub enum BasicType {
    String,
    Boolean,
    SignedInteger(u8),
    UnsignedInteger(u8),
    Float(u8),
}

/// Complex Type
#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub enum ComplexType {
    Array(BasicType),
    Set(BasicType),
    Map(BasicType, BasicType),
    Blob,
    Json,
    Xml,
    File,
    Folder,
    Path,
}
