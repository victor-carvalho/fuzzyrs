use std::char;

/// Mask of the value bits of a continuation byte.
const CONT_MASK: u8 = 0b0011_1111;

/// Returns the initial codepoint accumulator for the first byte.
/// The first byte is special, only want bottom 5 bits for width 2, 4 bits
/// for width 3, and 3 bits for width 4.
#[inline]
fn utf8_first_byte(byte: u8, width: u32) -> u32 {
    (byte & (0x7F >> width)) as u32
}

/// Returns the value of `ch` updated with continuation byte `byte`.
#[inline]
fn utf8_acc_cont_byte(ch: u32, byte: u8) -> u32 {
    (ch << 6) | (byte & CONT_MASK) as u32
}

#[inline]
fn unwrap_or_0(opt: Option<&u8>) -> u8 {
    match opt {
        Some(&byte) => byte,
        None => 0,
    }
}

/// Reads the next code point out of a byte iterator (assuming a
/// UTF-8-like encoding).
/// Copied from https://github.com/rust-lang/rust/blob/75b98fbe77d472d85d1691bae5b25e7eefb3609c/src/libcore/str/mod.rs#L515
#[inline]
pub fn next_code_point(bytes: &[u8]) -> Option<char> {
    let x = *bytes.get(0)?;
    if x < 128 {
        return Some(unsafe { char::from_u32_unchecked(x as u32) });
    }

    let init = utf8_first_byte(x, 2);
    let y = unwrap_or_0(bytes.get(1));
    let mut ch = utf8_acc_cont_byte(init, y);
    if x >= 0xE0 {
        let z = unwrap_or_0(bytes.get(2));
        let y_z = utf8_acc_cont_byte((y & CONT_MASK) as u32, z);
        ch = init << 12 | y_z;
        if x >= 0xF0 {
            let w = unwrap_or_0(bytes.get(3));
            ch = (init & 7) << 18 | utf8_acc_cont_byte(y_z, w);
        }
    }

    Some(unsafe { char::from_u32_unchecked(ch) })
}
