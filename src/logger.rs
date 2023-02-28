//! The logger implementation itself. These are implementation details not exposed in the public
//! API.

use log::{LevelFilter, Log};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// The environment variable for controlling the logging behavior.
const NIH_LOG_ENV: &str = "NIH_LOG";

/// The NIH-log logger. Construct one using the [`LoggerBuilder`].
pub struct Logger {
    /// The maximum log level filter. This is already set globally using [`log::set_max_level()`]
    /// but it's probably a good idea to check it again regardless.
    pub max_log_level: LevelFilter,
    /// The output target for the logger. This may be overwritten at runtime depending on the value
    /// of [`OutputTargetImpl::overwritable()`].
    pub output_target: OutputTargetImpl,
}

/// Similar to [`crate::builder::OutputTarget`], but contains the actual data needed to write to the
/// logger.
#[derive(Debug)]
pub enum OutputTargetImpl {
    /// The default logging target. On Windows this checks whether a Windows debugger is attached
    /// before logging. If there is a debugger, then the message is written using
    /// `OutputDebugStringA()`. Otherwise the message is written to STDERR instead. On non-Windows
    /// platforms this is equivalent to [`OutputTargetImpl`].
    ///
    /// # Notes
    ///
    /// This dynamic target is the only target that can be overwritten at runtime. The others are
    /// considered explicit choices and won't be overwritten.
    StderrOrWinDbg,
    /// Writes directly to STDERR.
    Stderr,
    /// Outputs to the Windows debugger using `OutputDebugStringA()`.
    WinDbg,
    /// Writes to the file.
    File(BufWriter<File>),
}

impl OutputTargetImpl {
    /// Whether a target was implicitly chosen and can be overwritten at runtime.
    pub fn overwritable(&self) -> bool {
        match self {
            OutputTargetImpl::StderrOrWinDbg => true,
            OutputTargetImpl::Stderr | OutputTargetImpl::WinDbg | OutputTargetImpl::File(_) => {
                false
            }
        }
    }

    /// Whether to use ANSI colors when writing to the target.
    pub fn colors(&self) -> bool {
        match self {
            OutputTargetImpl::Stderr
                if todo!("check if we're outputting to a supported terminal") =>
            {
                true
            }
            OutputTargetImpl::StderrOrWinDbg
                if todo!("not outputting to windb, and STDERR is from a supported terminal") =>
            {
                true
            }
            _ => false,
        }
    }

    /// If the `NIH_LOG` environment variable is set, then parse that according to the rules defined
    /// in the project's readme. Otherwise defaults to the dynamic `StderrOrWinDbg` target. If
    /// `NIH_LOG` is set to output to a file and the file couldn't be opened, then this will write
    /// the error to STDERR and then also fall back to `StderrOrWinDbg`.
    pub fn default_from_environment() -> Self {
        let nih_log_env = std::env::var(NIH_LOG_ENV);
        let nih_log_env_str = nih_log_env.as_deref().unwrap_or("");
        if nih_log_env_str.eq_ignore_ascii_case("stderr") {
            return Self::Stderr;
        }
        if nih_log_env_str.eq_ignore_ascii_case("windbg") {
            return Self::WinDbg;
        }
        if !nih_log_env_str.is_empty() {
            match Self::for_file_path(nih_log_env_str) {
                Ok(target) => return target,
                // TODO: Print this using the actual logger
                Err(err) => eprintln!(
                    "Could not open '{nih_log_env_str}' from NIH_LOG for logging, falling back to \
                     STDERR: {err}"
                ),
            }
        }

        Self::StderrOrWinDbg
    }

    pub fn for_file_path<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let file = File::options().create(true).append(true).open(path)?;

        Ok(Self::File(BufWriter::new(file)))
    }
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
