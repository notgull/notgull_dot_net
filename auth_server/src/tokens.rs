// GNU AGPL v3 License

/// Generate an authorization token.
#[inline]
pub fn generate_auth_token() -> String {
    let mut bytes: Vec<u8> = vec![0; 128];
    getrandom::getrandom(&mut bytes).expect("Failed to use system RNG");

    // clamp each byte to the ASCII range
    bytes.iter_mut().for_each(|b| {
        *b &= 0x7F;
        *b |= 0x10;
    });

    String::from_utf8(bytes).unwrap()
}
