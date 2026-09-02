/// Get a nicely formatted version of a binary string
pub fn fmt_bstr(mut s: &[u8]) -> String {
    let mut out = String::new();

    loop {
        match std::str::from_utf8(s) {
            Ok(valid) => {
                out.extend(valid.escape_default());
                break;
            }
            Err(error) => {
                let (valid, after_valid) = s.split_at(error.valid_up_to());
                out.extend(std::str::from_utf8(valid).unwrap().escape_default());

                let invalid_len = error.error_len().unwrap_or(after_valid.len());
                for &byte in &after_valid[0..=invalid_len] {
                    out.push_str(&format!(r"\x{:X}", byte));
                }

                s = &after_valid[invalid_len..];
            }
        }
    }

    out
}
