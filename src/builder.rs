///! A builder interface for the logger.
use log::LevelFilter;
use std::error::Error;
use std::fmt::Display;
use std::path::PathBuf;

use crate::logger::{Logger, OutputTargetImpl};
use crate::LOGGER_INSTANCE;

/// Constructs an NIH-log logger.
#[derive(Debug)]
pub struct LoggerBuilder {
    /// The maximum log level. Set when constructing the builder.
    pub max_log_level: LevelFilter,
    /// An explicitly set output target. When writing to a file this already contains the writer for
    /// the file to ensure that it can actually be written to when the logger is created.
    output_target: Option<OutputTargetImpl>,
}

/// Determines where the logger should write its output. If no explicit target is chosen, then a
/// default dynamic target is used instead. Check the readme for more information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputTarget {
    /// Write directly to STDERR.
    Stderr,
    /// Output to the Windows debugger using `OutputDebugStringA()`.
    // FIXME: Gate all uses of this behind the Windows platform
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
            output_target: None,
        }
    }

    /// Install the configured logger as the global logger. The global logger can only be set once.
    pub fn build_global(self) -> Result<(), SetLoggerError> {
        let max_log_level = self.max_log_level;
        let logger = Logger {
            max_log_level,
            // Picking an output target happens in three steps:
            // - If `LoggerBuilder::with_output_target()` was called, that target is used.
            // - If the `NIH_LOG` environment variable is non-empty, then that is parsed.
            // - Otherwise a dynamic target is used that writes to either STDERR or a WinDbg
            //   debugger depending on whether a Windows debugger is present.
            output_target: self
                .output_target
                .unwrap_or_else(OutputTargetImpl::default_from_environment),
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

    /// Explicitly set the otuput target for the logger. This is normally set using the `NIH_LOG`
    /// environment variable. If an explicit output target is set, then the output target cannot be
    /// changed anymore at runtime. Returns an error if the target could not be set.
    pub fn with_output_target(mut self, target: OutputTarget) -> Result<Self, SetTargetError> {
        self.output_target = Some(match target {
            OutputTarget::Stderr => OutputTargetImpl::Stderr(OutputTargetImpl::stderr_stream()),
            OutputTarget::WinDbg => OutputTargetImpl::WinDbg,
            OutputTarget::File(path) => match OutputTargetImpl::for_file_path(&path) {
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
