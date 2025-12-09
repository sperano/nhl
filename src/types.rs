//! Shared types used across the application
//!
//! This module contains type definitions that are shared between
//! the library (commands, tui) and the binary (main.rs).

#[allow(dead_code)]
enum TeamNameFormat {
    Abbreviated,
    City,
    Common,
    Full,
}
