pub mod loader;
pub mod schema;
pub mod validate;

pub use schema::{DecisionBand, PolicyFile, Rule, ThenClause, WhenClause};

pub fn scaffold_status() -> &'static str {
    "policy scaffold ready"
}
