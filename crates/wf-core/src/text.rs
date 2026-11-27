pub fn counted<T: Copy + PartialEq + From<u8> + std::fmt::Display>(
    count: T,
    singular: &str,
) -> String {
    let suffix = if count == T::from(1) { "" } else { "s" };
    format!("{count} {singular}{suffix}")
}

pub fn thousands(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if value < 0 {
        out.push('-');
    }
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

pub(crate) fn contains_word(text: &str, word: &str) -> bool {
    let text = text.to_lowercase();
    let word = word.to_lowercase();
    let boundary = |byte: Option<&u8>| match byte {
        Some(byte) => !byte.is_ascii_alphanumeric(),
        None => true,
    };
    text.match_indices(&word).any(|(start, found)| {
        let before = start.checked_sub(1).and_then(|at| text.as_bytes().get(at));
        boundary(before) && boundary(text.as_bytes().get(start + found.len()))
    })
}

#[cfg(test)]
mod tests {
    use super::{contains_word, counted, thousands};

    #[test]
    fn counted_words() {
        assert_eq!(counted(0_usize, "change"), "0 changes");
        assert_eq!(counted(1_usize, "change"), "1 change");
        assert_eq!(counted(2_usize, "change"), "2 changes");
        assert_eq!(counted(3_u32, "rank"), "3 ranks");
        assert_eq!(counted(2_i64, "act"), "2 acts");
        assert_eq!(counted(1.0_f64, "second"), "1 second");
        assert_eq!(counted(2.0_f64, "second"), "2 seconds");
    }

    #[test]
    fn whole_words_only() {
        assert!(contains_word("Braton Prime", "Braton"));
        assert!(contains_word("MK1-Braton", "braton"));
        assert!(!contains_word("Akbronco Prime", "Bronco"));
    }

    #[test]
    fn thousands_separators() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000), "1,000");
        assert_eq!(thousands(-1_234_567), "-1,234,567");
    }
}
