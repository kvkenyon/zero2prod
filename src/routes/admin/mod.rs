//! src/routes/admin/mod.rs
mod dashboard;
mod helpers;
mod logout;
mod password;

pub use dashboard::admin_dashboard;
pub use helpers::*;
pub use logout::*;
pub use password::*;
