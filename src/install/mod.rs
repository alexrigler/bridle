//! Installation management for bridle.

#![allow(dead_code)]
#![allow(unused_imports)]

pub mod discovery;
pub mod installer;
pub mod manifest;
pub mod types;
pub mod uninstaller;

pub use discovery::{discover_skills, DiscoveryError};
pub use types::*;
