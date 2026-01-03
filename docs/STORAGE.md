# Storage Specification

## Overview

CATHEDRAL storage is content-addressed with hash verification. All data is immutable and referenced by hash.

## Content Addressing

### Content Address

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentAddress {
    pub hash: Hash,
    pub algorithm: AddressAlgorithm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AddressAlgorithm {
    Blake3,
}

impl ContentAddress {
    pub fn from_data(data: &[u8]) -> Self {
        Self {
            hash: Hash::blake3(data),
            algorithm: AddressAlgorithm::Blake3,
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}:{}", self.algorithm.as_str(), self.hash)
    }

    pub fn from_str(s: &str) -> Result<Self, AddressError> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(AddressError::InvalidFormat);
        }

        let algorithm = match parts[0] {
            "blake3" => AddressAlgorithm::Blake3,
            _ => return Err(AddressError::UnknownAlgorithm),
        };

        let hash = Hash::from_str(parts[1])?;

        Ok(Self { hash, algorithm })
    }
}
```

### Hash

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash([u8; 32]);

impl Hash {
    pub fn blake3(data: &[u8]) -> Self {
        Self(blake3::hash(data).into())
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(hex: &str) -> Result<Self, HashError> {
        let bytes = hex::decode(hex)?;
        if bytes.len() != 32 {
            return Err(HashError::InvalidLength);
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(Self(array))
    }

    pub fn verify(&self, data: &[u8]) -> bool {
        Self::blake3(data) == *self
    }
}
```

## Blob Store

### Interface

```rust
pub trait ContentStore: Send + Sync {
    /// Store data and return content address
    fn put(&self, data: &[u8]) -> Result<ContentAddress, StoreError>;

    /// Retrieve data by content address
    fn get(&self, addr: &ContentAddress) -> Result<Vec<u8>, StoreError>;

    /// Check if content exists
    fn contains(&self, addr: &ContentAddress) -> Result<bool, StoreError>;

    /// Delete content (only if unreferenced)
    fn delete(&self, addr: &ContentAddress) -> Result<(), StoreError>;

    /// List all content addresses
    fn list(&self) -> Result<Vec<ContentAddress>, StoreError>;

    /// Get size of content in bytes
    fn size(&self, addr: &ContentAddress) -> Result<u64, StoreError>;
}
```

### Implementation (ReDB)

```rust
pub struct ReDbStore {
    db: redb::Database,
}

impl ReDbStore {
    pub fn new(path: &Path) -> Result<Self, StoreError> {
        let db = redb::Database::create(path)?;
        Ok(Self { db })
    }

    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let db = redb::Database::open(path)?;
        Ok(Self { db })
    }
}

impl ContentStore for ReDbStore {
    fn put(&self, data: &[u8]) -> Result<ContentAddress, StoreError> {
        let addr = ContentAddress::from_data(data);

        // Check if already exists
        if self.contains(&addr)? {
            return Ok(addr);
        }

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(BLOB_TABLE)?;
            table.insert(addr.to_string().as_str(), data)?;
        }
        write_txn.commit()?;

        Ok(addr)
    }

    fn get(&self, addr: &ContentAddress) -> Result<Vec<u8>, StoreError> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(BLOB_TABLE)?;

        let value = table.get(addr.to_string().as_str())?
            .ok_or(StoreError::NotFound)?;

        Ok.value().to_vec()
    }

    fn contains(&self, addr: &ContentAddress) -> Result<bool, StoreError> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(BLOB_TABLE)?;
        Ok(table.get(addr.to_string().as_str())?.is_some())
    }

    fn delete(&self, addr: &ContentAddress) -> Result<(), StoreError> {
        // Check if referenced
        if self.is_referenced(addr)? {
            return Err(StoreError::StillReferenced);
        }

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(BLOB_TABLE)?;
            table.remove(addr.to_string().as_str())?;
        }
        write_txn.commit()?;

        Ok(())
    }

    fn list(&self) -> Result<Vec<ContentAddress>, StoreError> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(BLOB_TABLE)?;

        let mut addrs = Vec::new();
        for result in table.iter() {
            let (key, _) = result?;
            addrs.push(ContentAddress::from_str(&key)?);
        }

        Ok(addrs)
    }

    fn size(&self, addr: &ContentAddress) -> Result<u64, StoreError> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(BLOB_TABLE)?;

        let value = table.get(addr.to_string().as_str())?
            .ok_or(StoreError::NotFound)?;

        Ok(value.value().len() as u64)
    }
}

impl ReDbStore {
    fn is_referenced(&self, addr: &ContentAddress) -> Result<bool, StoreError> {
        // Check reference table
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(REF_TABLE)?;

        Ok(table.get(addr.to_string().as_str())?.is_some())
    }
}

// Table definitions
const BLOB_TABLE: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("blobs");

const REF_TABLE: redb::TableDefinition<&str, redb::RedbKey> =
    redb::TableDefinition::new("references");
```

## Reference Tracking

### Reference Table

```rust
pub struct ReferenceTracker {
    store: ReDbStore,
}

impl ReferenceTracker {
    pub fn add_reference(&self, addr: ContentAddress, referrer: &str) -> Result<(), StoreError> {
        let write_txn = self.store.db.begin_write()?;
        {
            let mut table = write_txn.open_table(REF_TABLE)?;
            let key = addr.to_string();

            // Get existing references
            let mut refs: Vec<String> = table
                .get(&key)?
                .map(|v| serde_json::from_slice(v.value()).unwrap())
                .unwrap_or_default();

            refs.push(referrer.to_string());

            // Store updated references
            table.insert(&key, serde_json::to_vec(&refs).map_err(|_| StoreError::Serialization)?)?;
        }
        write_txn.commit()?;

        Ok(())
    }

    pub fn remove_reference(&self, addr: ContentAddress, referrer: &str) -> Result<(), StoreError> {
        let write_txn = self.store.db.begin_write()?;
        {
            let mut table = write_txn.open_table(REF_TABLE)?;
            let key = addr.to_string();

            if let Some(value) = table.get(&key)? {
                let mut refs: Vec<String> =
                    serde_json::from_slice(value.value()).unwrap();

                refs.retain(|r| r != referrer);

                if refs.is_empty() {
                    table.remove(&key)?;
                } else {
                    table.insert(&key, serde_json::to_vec(&refs).map_err(|_| StoreError::Serialization)?)?;
                }
            }
        }
        write_txn.commit()?;

        Ok(())
    }

    pub fn reference_count(&self, addr: ContentAddress) -> Result<usize, StoreError> {
        let read_txn = self.store.db.begin_read()?;
        let table = read_txn.open_table(REF_TABLE)?;

        match table.get(addr.to_string().as_str())? {
            Some(value) => {
                let refs: Vec<String> = serde_json::from_slice(value.value())?;
                Ok(refs.len())
            }
            None => Ok(0),
        }
    }
}
```

## Compaction

### Compaction Plan

```rust
pub struct CompactionPlan {
    pub unreferenced_blobs: Vec<ContentAddress>,
    pub total_size: u64,
    pub estimated_savings: u64,
}

pub struct Compactor {
    store: ReDbStore,