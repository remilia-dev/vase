/// A utility to build a `str` character by character.
pub struct StringBuilder {
    buffer: Vec<u8>,
    is_ascii: bool,
}
impl StringBuilder {
    /// Creates an empty string builder.
    pub fn new() -> StringBuilder {
        StringBuilder {
            buffer: Vec::with_capacity(30),
            is_ascii: true,
        }
    }
    /// Removes all characters in the buffer.
    ///
    /// `self.is_ascii()` will be true immediately after this call.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.is_ascii = true;
    }
    /// Adds a known-ASCII character to the buffer.
    /// This is *may* be faster than `append_char`.
    /// # Panics
    /// Panics when the ascii character is `>127`.
    pub fn append_ascii(&mut self, ascii: u8) {
        assert!(ascii.is_ascii());
        self.buffer.push(ascii);
    }
    /// Adds the given char to the buffer.
    pub fn append_char(&mut self, c: char) {
        if c.is_ascii() {
            self.buffer.push(c as u8);
        } else {
            self.is_ascii = false;
            let mut bytes = [0u8; 4];
            self.append_str(c.encode_utf8(&mut bytes));
        }
    }
    /// Returns a reference to the current buffer.
    pub fn current(&self) -> &str {
        let bytes = self.buffer.as_slice();
        // SAFETY: Only UTF-8 should have been appended to the buffer.
        // See the other functions for this type.
        return unsafe { std::str::from_utf8_unchecked(bytes) };
    }
    /// Boxes up the current buffer.
    ///
    /// This does not clear the buffer.
    pub fn current_as_box(&self) -> Box<str> {
        Box::from(self.current())
    }
    /// Returns whether the buffer is made of only ASCII or not.
    pub fn is_ascii(&self) -> bool {
        self.is_ascii
    }

    /// Adds all characters of the string to the buffer.
    /// # Note
    fn append_str(&mut self, string: &str) {
        self.buffer.reserve(string.len());
        for byte in string.as_bytes() {
            self.buffer.push(*byte);
        }
    }
}
impl Default for StringBuilder {
    fn default() -> Self {
        StringBuilder::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_builder_is_ascii() {
        assert!(StringBuilder::new().is_ascii());
    }

    #[test]
    fn clearing_builder_sets_is_ascii_to_true() {
        let mut builder = StringBuilder::new();
        builder.append_char('🌈');
        builder.clear();
        assert!(
            builder.is_ascii(),
            "StringBuilder.clear() did not properly reset is_ascii()."
        );
    }

    #[test]
    fn adding_ascii_char_does_not_change_is_ascii() {
        let mut builder = StringBuilder::new();
        for ascii_val in 0..=127u32 {
            builder.append_char(std::char::from_u32(ascii_val).unwrap())
        }
        assert!(builder.is_ascii());
    }

    #[test]
    fn adding_nonascii_char_sets_is_ascii_to_false() {
        let mut builder = StringBuilder::new();
        builder.append_char('🌈');
        assert!(!builder.is_ascii());
    }

    #[test]
    fn builder_properly_builds_a_string() {
        let mut builder = StringBuilder::new();
        builder.append_char('🌈');
        builder.append_ascii(b'r');
        assert_eq!(builder.current(), "🌈r");
        let boxed = builder.current_as_box();
        assert_eq!(boxed.as_ref(), "🌈r");
    }
}
