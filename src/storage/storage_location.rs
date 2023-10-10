/// Storage type
#[derive(Debug, Clone)]
pub enum StorageLocation {
    /// Store in memory
    Memory,
    
    /// Store in disk
    Disk,
}
