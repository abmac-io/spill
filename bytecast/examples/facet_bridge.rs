use bytecast::{BytecastFacet, DeriveFromBytes, DeriveToBytes};

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

    let wrapped = BytecastFacet::encode(&msg).unwrap();
    let json = facet_json::to_string(&wrapped).unwrap();
    println!("json: {json}");

    let decoded: BytecastFacet = facet_json::from_str(&json).unwrap();
    let msg2: Message = decoded.decode().unwrap();
    println!("deserialized: {msg2:?}");

    assert_eq!(msg, msg2);
}
