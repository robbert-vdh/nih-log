///! A builder interface for the logger.
use log::LevelFilter;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::logger::Logger;
use crate::target::OutputTargetImpl;
use crate::LOGGER_INSTANCE;

/// Constructs an NIH-log logger.
#[derive(Debug)]
pub struct LoggerBuilder {
    /// The maximum log level. Set when constructing the builder.
    max_log_level: LevelFilter,
    /// If set to `true`, then the module path is always shown. Useful for debug builds and to
    /// configure the module blacklist.
    always_show_module_path: bool,
    /// An explicitly set output target. If this is not set then the target is chosen based on the
    /// presence and contents of the `NIH_LOG` environment variable.
    output_target: Option<OutputTargetImpl>,
    /// Names of crates module paths that should be excluded from the log. Case sensitive, and only
    /// matches whole crate names and paths. Both the crate name and module path are checked
    /// separately to allow for a little bit of flexibility.
    module_blacklist: HashSet<String>,
}

/// Determines where the logger should write its output. If no explicit target is chosen, then a
/// default dynamic target is used instead. Check the readme for more information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputTarget {
    /// Write directly to STDERR.
    Stderr,
    /// Output to the Windows debugger using `OutputDebugString()`.
    #[cfg(windows)]
    WinDbg,
    /// Write the log output to a file.
    File(PathBuf),
    // TODO: Functions
}

/// An error raised when setting the logger's output target. This can be converted back to the
/// builder using `Into<Builder>`.
#[derive(Debug)]
pub enum SetTargetError {
    FileOpenError {
        builder: LoggerBuilder,
        path: PathBuf,
        error: std::io::Error,
    },
}

impl From<SetTargetError> for LoggerBuilder {
    fn from(value: SetTargetError) -> Self {
        match value {
            SetTargetError::FileOpenError { builder, .. } => builder,
        }
    }
}

impl Error for SetTargetError {}

impl Display for SetTargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetTargetError::FileOpenError {
                builder: _,
                path,
                error,
            } => {
                write!(f, "Could not open '{}' ({})", path.display(), error)
            }
        }
    }
}

/// An error raised when setting a logger after one has already been set.
// This is the same as `log::SetLoggerError`, except that we can create one ourselves.
#[derive(Debug)]
pub struct SetLoggerError(());

impl Error for SetLoggerError {}

impl Display for SetLoggerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tried to set a global logger after one has already been configured"
        )
    }
}

impl LoggerBuilder {
    /// Create a builder for a logger. The logger can be installed using the
    /// [`build_global()`][Self::build_global()] function.
    pub fn new(max_log_level: LevelFilter) -> Self {
        Self {
            max_log_level,
            always_show_module_path: false,
            output_target: None,
            module_blacklist: HashSet::new(),
        }
    }

    /// Install the configured logger as the global logger. The global logger can only be set once.
    pub fn build_global(self) -> Result<(), SetLoggerError> {
        let max_log_level = self.max_log_level;
        let always_show_module_path = self.always_show_module_path;
        let logger = Logger {
            max_log_level,
            always_show_module_path,
            // Picking an output target happens in three steps:
            // - If `LoggerBuilder::with_output_target()` was called, that target is used.
            // - If the `NIH_LOG` environment variable is non-empty, then that is parsed.
            // - Otherwise a dynamic target is used that writes to either STDERR or a WinDbg
            //   debugger depending on whether a Windows debugger is present.
            output_target: Mutex::new(
                self.output_target
                    .unwrap_or_else(OutputTargetImpl::default_from_environment),
            ),
            local_time_offset: time::UtcOffset::current_local_offset().unwrap_or_else(|_| {
                eprintln!("Could not get the local time offset, defaulting to UTC");
                time::UtcOffset::UTC
            }),

            module_blacklist: self.module_blacklist,
        };

        // We store a global logger instance and then set a static reference to that as the global
        // logger. This way we can access the global logger instance later if it needs to be
        // reconfigured at runtime
        match LOGGER_INSTANCE.try_insert(logger) {
            Ok(logger_instance) => {
                log::set_logger(logger_instance).map_err(|_| SetLoggerError(()))?;
                log::set_max_level(max_log_level);
                Ok(())
            }
            Err(_) => Err(SetLoggerError(())),
        }
    }

    /// Always show the module path. Normally this is only shown for the messages on the `Debug`
    /// level or on higher verbosity levels. Useful for debugging.
    pub fn always_show_module_path(mut self) -> Self {
        self.always_show_module_path = true;
        self
    }

    /// Filter out log messages produced by the given crate.
    pub fn filter_crate(mut self, crate_name: impl Into<String>) -> Self {
        self.module_blacklist.insert(crate_name.into());
        self
    }

    /// Filter out log messages produced by the given module. Module names are matched exactly and
    /// case sensitively. Filtering based on a module prefix is currently not supported.
    pub fn filter_module(mut self, crate_name: impl Into<String>) -> Self {
        // Right now both of these functions do the same thing, in the future we may want to
        // differentiate between them
        self.module_blacklist.insert(crate_name.into());
        self
    }

    /// Explicitly set the output target for the logger. This is normally set using the `NIH_LOG`
    /// environment variable. Returns an error if the target could not be set.
    #[allow(clippy::result_large_err)]
    pub fn with_output_target(mut self, target: OutputTarget) -> Result<Self, SetTargetError> {
        self.output_target = Some(match target {
            OutputTarget::Stderr => OutputTargetImpl::new_stderr(),
            #[cfg(windows)]
            OutputTarget::WinDbg => OutputTargetImpl::new_windbg(),
            OutputTarget::File(path) => match OutputTargetImpl::new_file_path(&path) {
                Ok(target) => target,
                Err(error) => {
                    return Err(SetTargetError::FileOpenError {
                        builder: self,
                        path,
                        error,
                    })
                }
            },
        });

        Ok(self)
    }
}
