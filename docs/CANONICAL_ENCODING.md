# Canonical Encoding Specification

## Overview

Canonical encoding ensures that the same data produces identical byte sequences across all platforms (Linux, macOS, Windows, x86_64, ARM64).

## Requirements

1. **Byte-identical output** - Same data → same bytes on all platforms
2. **Deterministic ordering** - No unstable ordering
3. **No hidden metadata** - No timestamps, random values, etc.
4. **Round-trip safe** - Encode → decode → encode = original bytes

## Encoding Format

We use `postcard` as the primary format:

```rust
pub fn encode_canonical<T: CanonicalEncode>(value: &T) -> Vec<u8> {
    postcard::to_allocvec(value).expect("encoding failed")
}
```

### Why Postcard?

- No floating point ambiguity
- Fixed-width integers
- Derivable for structs
- Compact representation
- Cross-platform stable

### Fallback: Custom CBOR

If postcard has issues, we implement custom CBOR:
- Explicit map ordering (sorted keys)
- Fixed tags for enum variants
- No undefined behavior

## Type Encoding Rules

### Primitives

| Type | Encoding |
|------|----------|
| `bool` | Single byte, 0x00 or 0x01 |
| `u8/u16/u32/u64` | Big-endian, fixed width |
| `i8/i16/i32/i64` | Two's complement, big-endian |
| `String` | UTF-8 bytes, length prefix |
| `bytes` | Length prefix + raw bytes |

### Collections

| Type | Encoding |
|------|----------|
| `Vec<T>` | Length + elements in order |
| `BTreeMap<K,V>` | Length + sorted (key, value) pairs |
| `[T; N]` | Fixed N, elements in order |
| `Option<T>` | Discriminant (0/1) + value if present |

### Forbidden Types

- `HashMap<K,V>` - Use `BTreeMap`
- `HashSet<T>` - Use `BTreeSet` or `Vec<T>` (sorted, deduplicated)
- `f32/f64` - Avoid, or encode as string with fixed precision
- `SystemTime` - Use `LogicalTime` instead

## Trait Definition

```rust
pub trait CanonicalEncode: serde::Serialize {
    fn encode(&self) -> Vec<u8> {
        postcard::to_allocvec(self).expect("canonical encoding failed")
    }

    fn encode_to_slice(&self, slice: &mut [u8]) -> Result<usize> {
        postcard::to_slice(self, slice).map_err(|_| CoreError::EncodingOverflow)
    }
}

pub trait CanonicalDecode<'de>: serde::Deserialize<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self>
    where
        Self: Sized,
    {
        postcard::from_bytes(bytes).map_err(|_| CoreError::InvalidEncoding)
    }
}

// Blanket impl for all Serialize/Deserialize types
impl<T: serde::Serialize> CanonicalEncode for T {}
impl<'de, T: serde::Deserialize<'de>> CanonicalDecode<'de> for T {}
```

## Validation

### Cross-Platform Test

```rust
// Test fixture generated on Linux x86_64
const ENCODED_EVENT: &[u8] = &[
    0x01, 0x02, 0x03, // ...
];

#[test]
fn test_encoding_matches_fixture() {
    let event = create_standard_test_event();
    let encoded = event.encode();
    assert_eq!(encoded, ENCODED_EVENT);
}
```

### CI Integration

Cross-platform test runs in CI:
1. Encode test data
2. Compare with fixture
3. Fail if mismatch

## Special Cases

### Floats

If floats are necessary:

```rust
#[derive(Serialize, Deserialize)]
pub struct FloatWrapper {
    #[serde(with = "serde_aux::field_attributes::serialize_with_string")]
    value: f64,
}
```

Or use fixed-point arithmetic:

```rust
pub type FixedDecimal = i64; // 4 decimal places
```

### Enums

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    RunCreated,  // Discriminant 0
    RunStarted,  // Discriminant 1
    // ...
}
```

### Timestamps

Use logical time, not wall clock:

```rust
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub struct LogicalTime(pub u64);
```

## Testing

### Property Tests

```rust
#[proptest]
fn test_encode_decode_roundtrip(val: TestValue) {
    let encoded = val.encode();
    let decoded = TestValue::decode(&encoded).unwrap();
    assert_eq!(decoded, val);
}

#[proptest]
fn test_encode_deterministic(val: TestValue) {
    let enc1 = val.encode();
    let enc2 = val.encode();
    assert_eq!(enc1, enc2);
}
```

### Fuzzing

```rust
fuzz_target!(|data: &[u8]| {
    if let Ok(val) = TestValue::decode(data) {
        let encoded = val.encode();
        assert_eq!(TestValue::decode(&encoded), Ok(val));
    }
});
```

## Performance

- Encode: ~100ns for 100-byte struct
- Decode: ~150ns for 100-byte struct
- Memory: Zero-copy on decode (when using postcard)

## Migration

If encoding must change:

1. Version the format
2. Support migration path
3. Update fixtures
4. Document breaking change

```rust
#[derive(Serialize, Deserialize)]
pub enum EncodingVersion {
    V1,
    V2,
}
```
