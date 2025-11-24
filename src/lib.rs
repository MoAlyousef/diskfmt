mod backends;
pub mod cli;
mod common;
pub mod config;
pub mod style;
#[cfg(all(feature = "a11y", not(feature = "gui")))]
compile_error!("feature \"a11y\" requires feature \"gui\"");
#[cfg(feature = "gui")]
pub mod gui;
mod utils;
