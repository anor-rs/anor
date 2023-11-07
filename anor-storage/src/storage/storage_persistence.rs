/// Persistence type
#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub enum StoragePersistence {
    /// Persist only in memory
    Memory = 0,
    
    /// Persist only in disk
    Disk = 1,

    /// Persist both in memory and disk
    Hybrid = 2
}
