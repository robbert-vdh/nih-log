//! Adapters for logging to a windows debugger. Split off into a module to avoid littering `#[cfg]`
//! attributes all over the place.

/// Whether the windows debugger is currently attached.
pub fn attached() -> bool {
    todo!()
}
