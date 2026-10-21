//! The steps of signing in by an emailed one-time code, apart from the widgets
//! that show them.

/// The most digits a one-time code has.
pub const CODE_LENGTH: usize = 6;

/// Where the emailed-code page is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// Asking which address the code is to be sent to.
    EmailAddress,
    /// Waiting for the code that was sent to `email`.
    EmailCode { email: String },
    /// The code was right, and the account asks for a second factor: its
    /// passcode, or a backup code. `code` is sent again with it.
    Passcode { email: String, code: String },
}

/// Whether `text` could be somebody's address: something, an `@`, then a domain
/// with a dot in it, and no spaces. The forum is the one to say whether it is a
/// member's.
pub fn is_valid_email(text: &str) -> bool {
    let Some((local, domain)) = text.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !text.contains(char::is_whitespace)
        && !domain.contains('@')
        && domain.split('.').count() >= 2
        && domain.split('.').all(|part| !part.is_empty())
}

/// What was typed as a code, without the spaces or dashes people group the digits
/// by.
pub fn clean_code(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace() && *c != '-').collect()
}

/// The step after an address is submitted: the code step for an address that
/// could be real, and a message for one that cannot.
pub fn submit_email(text: &str) -> Result<Step, &'static str> {
    let email = text.trim();
    if is_valid_email(email) {
        Ok(Step::EmailCode { email: email.to_owned() })
    } else {
        Err("Enter the email address of your account.")
    }
}

/// The code to check once one is submitted, or a message when what was typed is
/// not the shape of one.
pub fn submit_code(text: &str) -> Result<String, &'static str> {
    let code = clean_code(text);
    if code.len() == CODE_LENGTH && code.chars().all(|c| c.is_ascii_digit()) {
        Ok(code)
    } else {
        Err("The code is 6 digits.")
    }
}

/// The passcode to check once one is submitted, without the spaces or dashes
/// people group it by, or a message when nothing was typed.
pub fn submit_passcode(text: &str) -> Result<String, &'static str> {
    let passcode = clean_code(text);
    if passcode.is_empty() {
        Err("Enter your passcode.")
    } else {
        Ok(passcode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_passcode_is_whatever_was_typed_without_its_grouping() {
        assert_eq!(submit_passcode(" 123 456 "), Ok("123456".into()));
        assert_eq!(submit_passcode("ab12-cd34"), Ok("ab12cd34".into()));
        assert!(submit_passcode("  - ").is_err());
    }

    #[test]
    fn an_address_needs_a_name_an_at_and_a_dotted_domain() {
        assert!(is_valid_email("ferris@rust-lang.org"));
        assert!(is_valid_email("a.b+c@mail.example.co"));
        for bad in ["", "ferris", "@rust-lang.org", "ferris@", "ferris@rust", "ferris@rust.", "ferris@.org", "fer ris@rust-lang.org", "a@b@c.com"] {
            assert!(!is_valid_email(bad), "{bad:?}");
        }
    }

    #[test]
    fn a_submitted_address_is_trimmed_and_moves_on_to_the_code() {
        assert_eq!(
            submit_email("  ferris@rust-lang.org "),
            Ok(Step::EmailCode { email: "ferris@rust-lang.org".into() })
        );
        assert!(submit_email("ferris").is_err());
    }

    #[test]
    fn a_code_is_six_digits_however_they_are_grouped() {
        assert_eq!(submit_code("123456"), Ok("123456".into()));
        assert_eq!(submit_code(" 123 456 "), Ok("123456".into()));
        assert_eq!(submit_code("123-456"), Ok("123456".into()));
        for bad in ["", "12345", "1234567", "12345a", "abcdef"] {
            assert!(submit_code(bad).is_err(), "{bad:?}");
        }
    }
}
