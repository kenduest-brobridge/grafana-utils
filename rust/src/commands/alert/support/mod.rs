//! Thin facade for alert support utilities.
//!
//! Responsibility-specific implementations live in sibling files so the alert
//! domain can keep path, IO, normalization, policy, document, and scaffold
//! behavior isolated without changing the public `crate::alert_support::*`
//! surface.

#[path = "documents.rs"]
mod alert_support_documents;
#[path = "io.rs"]
mod alert_support_io;
#[path = "normalize.rs"]
mod alert_support_normalize;
#[path = "paths.rs"]
mod alert_support_paths;
#[path = "policy.rs"]
mod alert_support_policy;
#[path = "scaffold.rs"]
mod alert_support_scaffold;

pub use alert_support_documents::*;
pub use alert_support_io::*;
pub use alert_support_normalize::*;
pub use alert_support_paths::*;
pub use alert_support_policy::*;
pub use alert_support_scaffold::*;
