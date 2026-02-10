use bytecast::{DeriveFromBytes, DeriveToBytes, FromBytes, ToBytesExt as _};

#[derive(DeriveToBytes, DeriveFromBytes, Debug, PartialEq)]
struct Message {
    id: u32,
    text: String,
}

fn main() {
    let msg = Message {
        id: 42,
        text: String::from("hello bytecast"),
    };

    let bytes = msg.to_vec().unwrap();
    println!("serialized: {bytes:?} ({} bytes)", bytes.len());

    let (decoded, _) = Message::from_bytes(&bytes).unwrap();
    println!("deserialized: {decoded:?}");

    assert_eq!(msg, decoded);
}
