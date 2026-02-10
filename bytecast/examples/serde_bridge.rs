use bytecast::{BytecastSerde, DeriveFromBytes, DeriveToBytes};

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

    let wrapped = BytecastSerde(msg);
    let json = serde_json::to_string(&wrapped).unwrap();
    println!("json: {json}");

    let decoded: BytecastSerde<Message> = serde_json::from_str(&json).unwrap();
    println!("deserialized: {:?}", decoded.0);

    assert_eq!(wrapped.0, decoded.0);
}
