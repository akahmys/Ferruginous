/// The six-letter tag on a subsetted font's `/BaseFont`, if it carries one.
///
/// A font program that holds only the glyphs a document uses is named `ABCDEF+Original`:
/// exactly six uppercase letters, a `+`, and the original name. **This is the one place
/// that decides it.** Four sites used to decide it separately, in two disagreeing forms:
/// `base_font.contains('+')`, which calls `Arial+Bold` a subset, and a positional test at
/// byte 6 that accepted any six characters, digits and lowercase included.
///
/// The two forms separate no file in the 524-file corpus — all 316 subsetted names there
/// match both — so this unification fixes a divergence that had not yet been reached, not
/// a reading that was wrong today.
///
/// [ADR-0071](../../../docs/adr/0071-three-declarations-that-read-nothing-and-one-that-wrote-nothing.md).
pub fn subset_tag(base_font: &str) -> Option<&str> {
    let bytes = base_font.as_bytes();
    if bytes.len() < 8 || bytes[6] != b'+' {
        return None;
    }
    let tag = &base_font[..6];
    tag.bytes().all(|b| b.is_ascii_uppercase()).then_some(tag)
}

#[cfg(test)]
mod subset_tag_tests {
    use super::subset_tag;

    #[test]
    fn a_six_letter_tag_is_read() {
        assert_eq!(subset_tag("ARKUPQ+MSMincho"), Some("ARKUPQ"));
    }

    #[test]
    fn a_plus_elsewhere_in_the_name_is_not_a_tag() {
        assert_eq!(subset_tag("Arial+Bold"), None);
        assert_eq!(subset_tag("Helvetica"), None);
    }

    #[test]
    fn a_tag_that_is_not_six_uppercase_letters_is_not_a_tag() {
        assert_eq!(subset_tag("abcdef+Arial"), None);
        assert_eq!(subset_tag("ABC123+Arial"), None);
        assert_eq!(subset_tag("ABCDE+Arial"), None);
        assert_eq!(subset_tag("ABCDEFG+Arial"), None);
    }

    #[test]
    fn a_tag_with_nothing_after_it_is_not_a_tag() {
        assert_eq!(subset_tag("ABCDEF+"), None);
    }
}
