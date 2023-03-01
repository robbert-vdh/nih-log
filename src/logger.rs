//! The logger implementation itself. These are implementation details not exposed in the public
//! API.

use log::{LevelFilter, Log};

use crate::target::OutputTargetImpl;

/// The NIH-log logger. Construct one using the [`LoggerBuilder`].
pub struct Logger {
    /// The maximum log level filter. This is already set globally using [`log::set_max_level()`]
    /// but it's probably a good idea to check it again regardless.
    pub max_log_level: LevelFilter,
    /// The output target for the logger. This may be overwritten at runtime depending on the value
    /// of [`OutputTargetImpl::overwritable()`].
    pub output_target: OutputTargetImpl,
}

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        // TODO: Add crate/module filters
        metadata.level() <= self.max_log_level
    }

    fn log(&self, record: &log::Record) {
        todo!()
    }

    fn flush(&self) {
        todo!()
    }
}
