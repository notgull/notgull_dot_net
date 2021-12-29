// GNU AGPL v3 License

use getrandom::getrandom;

#[inline]
pub fn random_token() -> String {
    let mut bytes = vec![0u8; 16];
    getrandom(&mut bytes).unwrap();
    bytes.iter_mut().for_each(|b| {
        *b &= 0x7F;
        *b |= 0x10;
    });
    String::from_utf8(bytes).unwrap()
}
