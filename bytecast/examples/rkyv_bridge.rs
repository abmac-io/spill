use bytecast::{BytecastRkyv, DeriveFromBytes, DeriveToBytes};

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

    let wrapped = BytecastRkyv::encode(&msg).unwrap();
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&wrapped).unwrap();
    println!("rkyv bytes: {} bytes", bytes.len());

    let decoded = rkyv::from_bytes::<BytecastRkyv, rkyv::rancor::Error>(&bytes).unwrap();
    let msg2: Message = decoded.decode().unwrap();
    println!("deserialized: {msg2:?}");

    assert_eq!(msg, msg2);
}
