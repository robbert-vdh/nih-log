//! The logger's output targets.

use std::fmt::Debug;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use termcolor::{ColorChoice, StandardStream, WriteColor};

#[cfg(windows)]
mod windbg;

/// The environment variable for controlling the logging behavior.
const NIH_LOG_ENV: &str = "NIH_LOG";

/// Similar to [`crate::builder::OutputTarget`], but contains the actual data needed to write to the
/// logger.
pub enum OutputTargetImpl {
    /// The default logging target on Windows. This checks whether a Windows debugger is attached
    /// before logging. If there is a debugger, then the message is written using
    /// `OutputDebugString()`. Otherwise the message is written to STDERR instead.
    #[cfg(windows)]
    StderrOrWinDbg(BufWriter<StandardStream>, windbg::WinDbgWriter),
    /// Writes directly to STDERR. The default logging target on non-Windows platforms. May use
    /// colors colors depending on the environment.
    Stderr(BufWriter<StandardStream>),
    /// Outputs to the Windows debugger using `OutputDebugString()`.
    #[cfg(windows)]
    WinDbg(windbg::WinDbgWriter),
    /// Writes to the file.
    File(BufWriter<File>),
}

impl Debug for OutputTargetImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(windows)]
            OutputTargetImpl::StderrOrWinDbg(stderr, windbg) => f
                .debug_tuple("StderrOrWinDbg")
                .field(if stderr.get_ref().supports_color() {
                    &"<stderr stream with color support>"
                } else {
                    &"<stderr stream>"
                })
                .field(windbg)
                .finish(),
            OutputTargetImpl::Stderr(stderr) => f
                .debug_tuple("Stderr")
                .field(if stderr.get_ref().supports_color() {
                    &"<stderr stream with color support>"
                } else {
                    &"<stderr stream>"
                })
                .finish(),
            #[cfg(windows)]
            OutputTargetImpl::WinDbg(windbg) => f.debug_tuple("WinDbg").field(windbg).finish(),
            OutputTargetImpl::File(file) => f.debug_tuple("File").field(file).finish(),
        }
    }
}

impl OutputTargetImpl {
    /// Construct an [`OutputTargetImpl`] that writes to STDERR with optional color support
    /// determined by the environment. If a Windows debugger is attached when writing debug output,
    /// then the output is sent to the Windows debugger instead.
    #[cfg(windows)]
    pub fn new_stderr_or_windbg() -> Self {
        OutputTargetImpl::StderrOrWinDbg(
            BufWriter::with_capacity(1024, StandardStream::stderr(stderr_color_support())),
            windbg::WinDbgWriter::default(),
        )
    }

    /// Construct an [`OutputTargetImpl`] that writes to STDERR with optional color support
    /// determined by the environment.
    pub fn new_stderr() -> Self {
        OutputTargetImpl::Stderr(BufWriter::with_capacity(
            1024,
            StandardStream::stderr(stderr_color_support()),
        ))
    }

    /// Construct an [`OutputTargetImpl`] that writes to the Windows debugger.
    #[cfg(windows)]
    pub fn new_windbg() -> Self {
        OutputTargetImpl::WinDbg(windbg::WinDbgWriter::default())
    }

    /// Construct an [`OutputTargetImpl`] for doing buffered writes to a file.
    pub fn new_file_path<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let file = File::options().create(true).append(true).open(path)?;

        Ok(Self::File(BufWriter::with_capacity(1024, file)))
    }

    /// A writer that can be written to using the [`write!()`] and [`writeln!()`] macros. May
    /// perform a syscall to check whether the Windows debugger is attached so this should be reused
    /// for multiple `write!()` calls.
    pub fn writer(&mut self) -> &mut dyn Write {
        match self {
            #[cfg(windows)]
            OutputTargetImpl::StderrOrWinDbg(_, ref mut windbg) if windbg::attached() => windbg,
            #[cfg(windows)]
            OutputTargetImpl::StderrOrWinDbg(ref mut stderr, _) => stderr,
            OutputTargetImpl::Stderr(ref mut stderr) => stderr,
            #[cfg(windows)]
            OutputTargetImpl::WinDbg(ref mut windbg) => windbg,
            OutputTargetImpl::File(ref mut file) => file,
        }
    }

    /// The color writer for writing terminal colors. Returns `None` if [`writer()`][Self::writer()]
    /// would return a writer for anything other than an STDERR stream.
    pub fn color_writer(&mut self) -> Option<&mut dyn WriteColor> {
        match self {
            #[cfg(windows)]
            OutputTargetImpl::StderrOrWinDbg(ref mut stderr, _) if !windbg::attached() => {
                Some(stderr.get_mut())
            }
            #[cfg(windows)]
            OutputTargetImpl::StderrOrWinDbg(_, _) => None,
            OutputTargetImpl::Stderr(ref mut stderr) => Some(stderr.get_mut()),
            #[cfg(windows)]
            OutputTargetImpl::WinDbg(_) => None,
            OutputTargetImpl::File(_) => None,
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
            return Self::new_stderr();
        }
        #[cfg(windows)]
        if nih_log_env_str.eq_ignore_ascii_case("windbg") {
            return Self::new_windbg();
        }
        if !nih_log_env_str.is_empty() {
            match Self::new_file_path(nih_log_env_str) {
                Ok(target) => return target,
                // TODO: Print this using the actual logger
                Err(err) => eprintln!(
                    "Could not open '{nih_log_env_str}' from NIH_LOG for logging, falling back to \
                     STDERR: {err}"
                ),
            }
        }

        #[cfg(windows)]
        return Self::new_stderr_or_windbg();
        #[cfg(not(windows))]
        return Self::new_stderr();
    }
}

/// Whether to use colors when outputting to STDERR. Considers the `CLICOLOR`, `CLICOLOR_FORCE`, and
/// `NO_COLOR` environment variables, and whether or not STDERR is attached to a real TTY.
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
