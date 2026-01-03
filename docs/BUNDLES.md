# Bundle Format Specification

## Overview

Bundles are portable, self-contained packages for reproducing CATHEDRAL runs.

## Bundle Structure

```
bundle.cath-bundle/
├── MANIFEST.json           # Bundle metadata and hashes
├── metadata.json           # Run metadata
├── workflow.cath           # Original workflow definition
├── dag.json                # Compiled DAG
├── events.cath-log         # Event log
├── snapshot.cath-snap      # Optional starting snapshot
└── blobs/                  # Content-addressed blob store
    ├── abc123...           # Hash-named blob files
    ├── def456...
    └── ...
```

## Manifest

```json
{
    "bundle_version": "1.0",
    "bundle_id": "bundle_abc123",
    "created_at": "2025-01-15T10:30:00Z",
    "format": "cath-bundle",
    "files": {
        "metadata.json": {
            "hash": "def456...",
            "size": 1234
        },
        "workflow.cath": {
            "hash": "ghi789...",
            "size": 567
        },
        "dag.json": {
            "hash": "jkl012...",
            "size": 890
        },
        "events.cath-log": {
            "hash": "mno345...",
            "size": 12345
        },
        "snapshot.cath-snap": {
            "hash": "pqr678...",
            "size": 67890,
            "optional": true
        }
    },
    "blob_count": 42,
    "signature": "sig_stu901..."
}
```

## Metadata

```json
{
    "run_id": "run_001",
    "workflow_name": "data_pipeline",
    "workflow_version": "1.0.0",
    "started_at": "2025-01-15T10:00:00Z",
    "completed_at": "2025-01-15T10:05:00Z",
    "status": "completed",
    "node_count": 5,
    "event_count": 1234,
    "platform": {
        "os": "linux",
        "arch": "x86_64"
    },
    "cathedral_version": "0.1.0"
}
```

## Event Log Format

### Binary Encoding

Events are stored in canonical binary encoding for efficiency:

```rust
pub struct EventLog {
    events: Vec<Event>,
    encoding: Encoding,
}

#[derive(Clone, Copy)]
pub enum Encoding {
    Postcard,
    Cbor,
}

impl EventLog {
    pub fn write_to(&self, path: &Path) -> Result<(), IoError> {
        let mut file = BufWriter::new(File::create(path)?);

        // Write header
        file.write_all(b"CATHLOG")?;
        file.write_all(&(1u32).to_be_bytes())?;  // Version
        file.write_all(&(self.events.len() as u32).to_be_bytes())?;

        // Write events
        for event in &self.events {
            let encoded = postcard::to_allocvec(event)?;
            file.write_all(&(encoded.len() as u32).to_be_bytes())?;
            file.write_all(&encoded)?;
        }

        file.flush()?;
        Ok(())
    }

    pub fn read_from(path: &Path) -> Result<Self, IoError> {
        let mut file = BufReader::new(File::open(path)?);

        // Read and verify header
        let mut magic = [0u8; 6];
        file.read_exact(&mut magic)?;
        if magic != b"CATHLOG"[..] {
            return Err(IoError::InvalidFormat);
        }

        let mut version = [0u8; 4];
        file.read_exact(&mut version)?;
        let version = u32::from_be_bytes(version);
        if version != 1 {
            return Err(IoError::UnsupportedVersion(version));
        }

        // Read event count
        let mut count_bytes = [0u8; 4];
        file.read_exact(&mut count_bytes)?;
        let count = u32::from_be_bytes(count_bytes) as usize;

        // Read events
        let mut events = Vec::with_capacity(count);
        for _ in 0..count {
            let mut len_bytes = [0u8; 4];
            file.read_exact(&mut len_bytes)?;
            let len = u32::from_be_bytes(len_bytes) as usize;

            let mut encoded = vec![0u8; len];
            file.read_exact(&mut encoded)?;
            let event: Event = postcard::from_bytes(&encoded)?;
            events.push(event);
        }

        Ok(Self { events, encoding: Encoding::Postcard })
    }
}
```

### Index

For fast random access, an index is maintained:

```rust
pub struct EventIndex {
    offsets: Vec<(u64, u64)>,  // (event_id, byte_offset)
}

impl EventIndex {
    pub fn build(log: &EventLog) -> Self {
        let mut index = EventIndex { offsets: Vec::new() };
        let mut offset = 0u64;

        for event in &log.events {
            index.offsets.push((event.id, offset));
            offset += event.encoded_size() as u64;
        }

        index
    }

    pub fn get_offset(&self, event_id: u64) -> Option<u64> {
        self.offsets
            .binary_search_by_key(&event_id, |(id, _)| *id)
            .ok()
            .map(|i| self.offsets[i].1)
    }
}
```

## Blob Store

### Blob Naming

Blobs are stored as files named by their content hash:

```
blobs/
├── ab/  # First two hex chars of hash
│   └── cdef1234...  # Full hash
├── 12/
│   └── 3456abcd...
```

### Blob Format

```rust
pub struct BlobFile {
    pub header: BlobHeader,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct BlobHeader {
    pub format: BlobFormat,
    pub compression: Option<Compression>,
    pub hash: Hash,
    pub size: u64,
}

#[derive(Serialize, Deserialize)]
pub enum BlobFormat {
    Raw,
    Json,
    Cbor,
    Postcard,
}
```

## Compression

Blobs can be compressed for storage:

```rust
pub fn compress_blob(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
    use zstd::stream::encode_all;

    let compressed = encode_all(data, 3)?;  // Level 3
    Ok(compressed)
}

pub fn decompress_blob(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
    use zstd::stream::decode_all;

    let decompressed = decode_all(data)?;
    Ok(decompressed)
}
```

## Bundle Creation

```rust
pub struct BundleBuilder {
    run_id: RunId,
    events: Vec<Event>,
    blobs: BTreeMap<Hash, Vec<u8>>,
    snapshot: Option<Snapshot>,
    dag: Dag,
    workflow: String,
}

impl BundleBuilder {
    pub fn new(run_id: RunId) -> Self {
        Self {
            run_id,
            events: Vec::new(),
            blobs: BTreeMap::new(),
            snapshot: None,
            dag: Dag::new(),
            workflow: String::new(),
        }
    }

    pub fn add_event(&mut self, event: Event) {
        self.events.push(event);
    }

    pub fn add_blob(&mut self, data: Vec<u8>) -> Hash {
        let hash = Hash::compute(&data);
        self.blobs.insert(hash, data);
        hash
    }

    pub fn build(self, output: &Path) -> Result<BundleError> {
        // Create bundle directory
        let bundle_dir = output;
        fs::create_dir_all(bundle_dir.join("blobs"))?;

        // Write manifest
        let manifest = self.create_manifest()?;
        fs::write(
            bundle_dir.join("MANIFEST.json"),
            serde_json::to_vec_pretty(&manifest)?
        )?;

        // Write metadata
        fs::write(
            bundle_dir.join("metadata.json"),
            serde_json::to_vec_pretty(&self.metadata)?
        )?;

        // Write workflow
        fs::write(bundle_dir.join("workflow.cath"), &self.workflow)?;

        // Write DAG
        fs::write(
            bundle_dir.join("dag.json"),
            serde_json::to_vec_pretty(&self.dag)?
        )?;

        // Write events
        let log = EventLog::from_events(self.events);
        log.write_to(bundle_dir.join("events.cath-log"))?;

        // Write blobs
        for (hash, data) in &self.blobs {
            let blob_path = blob_path(hash);
            fs::write(bundle_dir.join(&blob_path), data)?;
        }

        // Write snapshot if present
        if let Some(snapshot) = &self.snapshot {
            fs::write(
                bundle_dir.join("snapshot.cath-snap"),
                postcard::to_allocvec(snapshot)?
            )?;
        }

        Ok(())
    }
}
```

## Bundle Loading

```rust
pub struct BundleLoader;

impl BundleLoader {
    pub fn load(path: &Path) -> Result<ReplayBundle, BundleError> {
        // Verify manifest
        let manifest = self.load_manifest(path)?;

        // Verify file hashes
        self.verify_hashes(path, &manifest)?;

        // Load metadata
        let metadata = self.load_metadata(path)?;

        // Load events
        let events = self.load_events(path)?;

        // Load blobs
        let blobs = self.load_blobs(path)?;

        // Load snapshot if present
        let snapshot = self.load_snapshot(path)?;

        Ok(ReplayBundle {
            metadata,
            events,
            blobs,
            snapshot,
        })
    }

    fn verify_hashes(&self, path: &Path, manifest: &BundleManifest) -> Result<(), BundleError> {
        for (file_name, file_info) in &manifest.files {
            if file_info.optional && !path.join(file_name).exists() {
                continue;
            }

            let data = fs::read(path.join(file_name))?;
            let hash = Hash::compute(&data);

            if hash != file_info.hash {
                return Err(BundleError::HashMismatch {
                    file: file_name.clone(),
                    expected: file_info.hash,
                    actual: hash,
                });
            }
        }

        Ok(())
    }
}
```

## CLI Commands

### Create Bundle

```bash
cathedral bundle --run run-001 --output run-001.cath-bundle
```

### Verify Bundle

```bash
cathedral verify-bundle --bundle run-001.cath-bundle
```

### Extract from Bundle

```bash
# Extract events
cathedral inspect --bundle run-001.cath-bundle --events --output events.json

# Extract specific blob
cathedral inspect --bundle run-001.cath-bundle --blob abc123... --output blob.bin

# Extract snapshot
cathedral inspect --bundle run-001.cath-bundle --snapshot --output snap.json
```

## Performance

- Bundle creation: O(events + blobs)
- Bundle loading: O(events)
- Hash verification: Parallelizable over files
- Compression: 3-5x reduction for typical data
