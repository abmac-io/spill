use bytecast::{FromBytes, Immutable, IntoBytes, KnownLayout, ToBytes, ZcFromBytes, ZeroCopyType};

#[derive(ZcFromBytes, IntoBytes, Immutable, KnownLayout, Debug, PartialEq)]
#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

impl ZeroCopyType for Point {}

fn main() {
    let p = Point { x: 10, y: 20 };
    let mut buf = [0u8; 8];
    p.to_bytes(&mut buf).unwrap();
    println!("serialized: {buf:?}");

    let (decoded, _) = Point::from_bytes(&buf).unwrap();
    println!("deserialized: {decoded:?}");

    assert_eq!(p, decoded);
}
