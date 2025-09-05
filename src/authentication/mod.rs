//! src/authentication/mod.rs

mod middleware;
mod password;
pub use middleware::UserId;
pub use middleware::reject_anonymous_users;
pub use password::*;
