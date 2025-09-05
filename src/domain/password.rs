//! src/domain/password.rs
use secrecy::{ExposeSecret, Secret};
use std::fmt::Write;
use zxcvbn::zxcvbn;

#[derive(Debug)]
pub struct Password(Secret<String>);

impl Password {
    pub fn parse(password: Secret<String>) -> Result<Self, String> {
        let entropy = zxcvbn(password.expose_secret(), &[]).map_err(|e| e.to_string())?;

        if entropy.score() < 3 {
            let feedback = entropy.feedback();

            let err_msg = match feedback {
                Some(feedback) => {
                    let mut err_msg = String::new();
                    if let Some(warning) = feedback.warning() {
                        writeln!(err_msg, "{}", warning).unwrap();
                    }

                    for suggestion in feedback.suggestions() {
                        writeln!(err_msg, "{}", suggestion).unwrap();
                    }

                    err_msg
                }
                None => "The password is too weak.".to_string(),
            };
            return Err(err_msg);
        }

        Ok(Self(password))
    }

    pub fn password(&self) -> &Secret<String> {
        &self.0
    }
}

impl std::fmt::Display for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", "*".repeat(self.password().expose_secret().len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use secrecy::Secret;

    #[test]
    fn weak_password_fails_parse() {
        let weak_password = Secret::new("abc123".to_string());
        let password = Password::parse(weak_password);
        assert_err!(password);
    }

    #[test]
    fn high_entropy_password_parses_successfully() {
        let high_entropy_password = Secret::new("r0sebudmaelstrom11/20/91aaaa".to_string());
        let password = Password::parse(high_entropy_password);
        assert_ok!(password);
    }

    #[test]
    fn password_with_entropy_2_fails_to_parse() {
        let entropy_2_password = Secret::new("hey<123".to_string());
        let password = Password::parse(entropy_2_password);
        assert_err!(password);
    }
}
