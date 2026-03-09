pub mod artifact;
pub mod derive;

pub use artifact::{ArtifactBasisEntry, ArtifactRefusal, ArtifactReport};

pub fn scaffold_status() -> &'static str {
    "bundle scaffold ready"
}
