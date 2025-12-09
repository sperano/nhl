/// Development utilities module
///
/// This module contains utilities for development and debugging,
/// such as screenshot capture and mock data providers.

#[cfg(feature = "development")]
pub mod screenshot;

#[cfg(any(test, feature = "development"))]
pub mod mock_client;
