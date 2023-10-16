/// Storage type
#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub enum StorageLocation {
    /// Store in memory
    Memory,
    
    /// Store in disk
    Disk,
}
