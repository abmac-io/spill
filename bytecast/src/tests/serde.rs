use alloc::string::String;

use crate::BytecastSerde;

// Round-trip u32 through JSON (human-readable, base64)
#[test]
fn test_serde_json_u32_roundtrip() {
    let original = BytecastSerde(0x12345678u32);
    let json = serde_json::to_string(&original).unwrap();

    // Should be a base64 string
    assert!(json.starts_with('"'));
    assert!(json.ends_with('"'));

    let decoded: BytecastSerde<u32> = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.0, 0x12345678u32);
}

// Round-trip String through JSON
#[test]
fn test_serde_json_string_roundtrip() {
    let original = BytecastSerde(String::from("hello world"));
    let json = serde_json::to_string(&original).unwrap();

    let decoded: BytecastSerde<String> = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.0, "hello world");
}

// Round-trip bool through JSON
#[test]
fn test_serde_json_bool_roundtrip() {
    let original = BytecastSerde(true);
    let json = serde_json::to_string(&original).unwrap();

    let decoded: BytecastSerde<bool> = serde_json::from_str(&json).unwrap();
    assert!(decoded.0);
}

// Invalid base64 should produce an error
#[test]
fn test_serde_json_invalid_base64() {
    let result: Result<BytecastSerde<u32>, _> = serde_json::from_str("\"not-valid-base64!!!\"");
    assert!(result.is_err());
}

// Truncated bytes should produce an error
#[test]
fn test_serde_json_truncated_bytes() {
    use base64::Engine;
    // Encode only 2 bytes, but u32 needs 4
    let short = base64::engine::general_purpose::STANDARD.encode([0u8, 1]);
    let json = alloc::format!("\"{}\"", short);

    let result: Result<BytecastSerde<u32>, _> = serde_json::from_str(&json);
    assert!(result.is_err());
}
