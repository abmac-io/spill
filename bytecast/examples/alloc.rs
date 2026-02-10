use bytecast::{FromBytes, ToBytes, ToBytesExt};

fn main() {
    // Vec<T>
    let data: Vec<u8> = vec![1, 2, 3, 4, 5];
    let bytes = ToBytesExt::to_vec(&data).unwrap();
    let (decoded, _) = Vec::<u8>::from_bytes(&bytes).unwrap();
    println!("Vec<u8>: {decoded:?}");
    assert_eq!(decoded, data);

    // String
    let text = String::from("hello bytecast");
    let bytes = ToBytesExt::to_vec(&text).unwrap();
    let (decoded, _) = String::from_bytes(&bytes).unwrap();
    println!("String: {decoded:?}");
    assert_eq!(decoded, text);

    // Fixed-size buffer still works too
    let value: u32 = 0x12345678;
    let mut buf = [0u8; 4];
    value.to_bytes(&mut buf).unwrap();
    let (decoded, _) = u32::from_bytes(&buf).unwrap();
    println!("u32: {decoded:#x}");
    assert_eq!(decoded, value);
}
