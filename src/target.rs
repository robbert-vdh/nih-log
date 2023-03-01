//! The logger's output targets.

use std::fmt::Debug;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use termcolor::{BufferedStandardStream, ColorChoice, WriteColor};

/// The environment variable for controlling the logging behavior.
const NIH_LOG_ENV: &str = "NIH_LOG";

/// Similar to [`crate::builder::OutputTarget`], but contains the actual data needed to write to the
/// logger.
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
    StderrOrWinDbg(BufferedStandardStream),
    /// Writes directly to STDERR.
    Stderr(BufferedStandardStream),
    /// Outputs to the Windows debugger using `OutputDebugStringA()`.
    WinDbg,
    /// Writes to the file.
    File(BufWriter<File>),
}

impl Debug for OutputTargetImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StderrOrWinDbg(stderr) => f
                .debug_tuple("StderrOrWinDbg")
                .field(if stderr.supports_color() {
                    &"<stderr stream with color support>"
                } else {
                    &"<stderr stream>"
                })
                .finish(),
            Self::Stderr(stderr) => f
                .debug_tuple("Stderr")
                .field(if stderr.supports_color() {
                    &"<stderr stream with color support>"
                } else {
                    &"<stderr stream>"
                })
                .finish(),
            Self::WinDbg => write!(f, "WinDbg"),
            Self::File(file) => f.debug_tuple("File").field(file).finish(),
        }
    }
}

impl OutputTargetImpl {
    /// Construct an [`OutputTargetImpl`] for doing buffered writes to a file.
    pub fn for_file_path<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let file = File::options().create(true).append(true).open(path)?;

        Ok(Self::File(BufWriter::new(file)))
    }

    /// Construct a [`BufferedStandardStream`] that writes to STDERR with optional color support
    /// determined by the environment.
    pub fn stderr_stream() -> BufferedStandardStream {
        BufferedStandardStream::stderr(stderr_color_support())
    }

    /// A writer that can be written to using the [`write!()`] and [`writeln!()`] macros. May
    /// perform a syscall to check whether the Windows debugger is attached so this should be reused
    /// for multiple `write!()` calls.
    pub fn writer(&mut self) -> &mut dyn Write {
        match self {
            OutputTargetImpl::StderrOrWinDbg(ref mut stderr) if !windbg_attached() => stderr,
            OutputTargetImpl::Stderr(ref mut stderr) => stderr,
            OutputTargetImpl::StderrOrWinDbg(_) => todo!("windbg"),
            OutputTargetImpl::WinDbg => todo!("windbg"),
            OutputTargetImpl::File(ref mut file) => file,
        }
    }

    /// The color writer for writing terminal colors. Returns `None` if [`writer()`][Self::writer()]
    /// would return a writer for anything other than an STDERR stream.
    pub fn color_writer(&mut self) -> Option<&mut dyn WriteColor> {
        match self {
            OutputTargetImpl::StderrOrWinDbg(ref mut stderr) if !windbg_attached() => Some(stderr),
            OutputTargetImpl::Stderr(ref mut stderr) => Some(stderr),
            OutputTargetImpl::StderrOrWinDbg(_)
            | OutputTargetImpl::WinDbg
            | OutputTargetImpl::File(_) => None,
        }
    }

    /// Whether a target was implicitly chosen and can be overwritten at runtime.
    pub fn overwritable(&self) -> bool {
        match self {
            OutputTargetImpl::StderrOrWinDbg(_) => true,
            OutputTargetImpl::Stderr(_) | OutputTargetImpl::WinDbg | OutputTargetImpl::File(_) => {
                false
            }
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
            return Self::Stderr(Self::stderr_stream());
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

        Self::StderrOrWinDbg(Self::stderr_stream())
    }
}

/// Whether to use colors when outputting to STDERR. Considers the `CLICOLOR`, `CLICOLOR_FORCE`,
/// and `NO_COLOR` environment variables, and whether or not STDERR is attached to a real TTY.
fn stderr_color_support() -> ColorChoice {
    if let Ok(value) = std::env::var("CLICOLOR_FORCE") {
        if value.trim() != "0" {
            return ColorChoice::Always;
        }
    }

    if let Ok(value) = std::env::var("NO_COLOR") {
        if value.trim() != "0" {
            return ColorChoice::Never;
        }
    }

    if let Ok(value) = std::env::var("CLICOLOR") {
        if value.trim() == "0" {
            return ColorChoice::Never;
        }
    }

    // If `CLICOLOR` is unset or set to a truthy value, and colors aren't forced, then terminal
    // support determines whether or not colors are used
    if atty::is(atty::Stream::Stderr) {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    }
}

fn windbg_attached() -> bool {
    todo!()
}
