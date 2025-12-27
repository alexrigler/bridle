//! Configuration management for bridle.

#![allow(dead_code)]
#![allow(unused_imports)]

mod bridle;
mod manager;
mod profile_name;

pub use bridle::BridleConfig;
pub use manager::ProfileManager;
pub use profile_name::{InvalidProfileName, ProfileName};
