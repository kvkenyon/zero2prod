//! src/domain/subscriber_email.rs
use validator::ValidateEmail;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(email: String) -> Result<SubscriberEmail, String> {
        // TODO: Implement validation
        if email.validate_email() {
            Ok(Self(email))
        } else {
            Err(format!("{} is an invalid email address.", email))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SubscriberEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use claims::assert_err;
    use fake::Fake;
    use fake::faker::internet::en::SafeEmail;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut rng = StdRng::seed_from_u64(u64::arbitrary(g));
            let email = SafeEmail().fake_with_rng(&mut rng);
            Self(email)
        }
    }

    #[quickcheck_macros::quickcheck()]
    fn valid_emails_are_parsed_successfully(valid_email: ValidEmailFixture) -> bool {
        dbg!(&valid_email.0);
        SubscriberEmail::parse(valid_email.0).is_ok()
    }

    #[test]
    fn an_empty_string_fails() {
        assert_err!(SubscriberEmail::parse("".to_string()));
    }

    #[test]
    fn whitespace_only_email_fails() {
        assert_err!(SubscriberEmail::parse("  ".to_string()));
    }

    #[test]
    fn blatant_non_email_fails() {
        assert_err!(SubscriberEmail::parse("not an email".to_string()));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "kevin.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }
    #[test]
    fn email_missing_name_is_rejected() {
        let email = "@kevin.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }
}
