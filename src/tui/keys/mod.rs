//! Keyboard event to action mapping
//!
//! This module handles converting crossterm KeyEvents into framework Actions.
//! It contains all the keyboard navigation logic for the TUI.

#[cfg(feature = "development")]
mod demo;
mod dispatch;
mod focus;
mod global;
mod scores;
mod settings;
mod standings;

#[cfg(test)]
mod tests;

pub use dispatch::key_to_action;
