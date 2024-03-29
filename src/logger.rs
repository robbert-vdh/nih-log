//! The logger implementation itself. These are implementation details not exposed in the public
//! API.

use log::{Level, LevelFilter, Log};
use std::cell::Cell;
use std::collections::HashSet;
use std::sync::Mutex;
use termcolor::Color;
use time::UtcOffset;

use crate::target::{OutputTargetImpl, WriteExt};

/// The formatting description for times. Each log message is prefixed by the current time as
/// `hh:mm:ss`.
const TIME_FORMAT_DESCRIPTION: &[time::format_description::FormatItem] =
    time::macros::format_description!("[hour]:[minute]:[second]");

thread_local! {
    static IS_REENTRANT_LOGGING_CALL: Cell<bool> = Cell::new(false);
}

/// The NIH-log logger. Construct one using the [`LoggerBuilder`].
pub struct Logger {
    /// The maximum log level filter. This is already set globally using [`log::set_max_level()`]
    /// but it's probably a good idea to check it again regardless.
    pub max_log_level: LevelFilter,
    /// If set to `true`, then the module path is always shown. Useful for debug builds and to
    /// configure the module blacklist.
    pub always_show_module_path: bool,
    /// The local time offset. Queried once at startup to avoid having to do this over and over
    /// again.
    pub local_time_offset: UtcOffset,
    /// The output target for the logger.
    pub output_target: Mutex<OutputTargetImpl>,
    /// Names of crates module paths that should be excluded from the log. Case sensitive, and only
    /// matches whole crate names and paths. Both the crate name and module path are checked
    /// separately to allow for a little bit of flexibility.
    pub module_blacklist: HashSet<String>,
}

impl Logger {
    /// Check if a target is enabled by comparing it to `self.module_blacklist`. If it contains a
    /// colon, also check if the first part (assumed to be a crate name) matches the blacklist.
    pub fn target_enabled(&self, target: &str) -> bool {
        // The filtering happens by both the crate and module name. We don't have very sophisticated
        // filtering needs, so let's keep this simple and performant.
        if let Some((crate_name, _)) = target.split_once(':') {
            if self.module_blacklist.contains(crate_name) {
                return false;
            }
        }

        !self.module_blacklist.contains(target)
    }

    fn do_log(&self, mut writer: &mut dyn WriteExt, record: &log::Record) {
        // The log message consists of the following elements:
        // 1) The current time in `hh:mm:ss`
        // 2) The log level, colored if colors are enabled
        // 3) (only on the debug and trace levels) The ID of the current thread
        // 4) (only on the debug and trace levels) The crate and module path
        // 5) (only on the trace level) The file name and line number
        // 6) The actual log message
        // TODO: We silently ignore failing writes and flushes. Is there anything reasonable we can
        //       do here other than panicking? (which isn't super reasonable)
        let current_time = time::OffsetDateTime::now_utc().to_offset(self.local_time_offset);
        let _ = current_time.format_into(&mut writer, TIME_FORMAT_DESCRIPTION);

        // If `writer` is a STDERR stream that outputs to a terminal with color support, we can
        // colorize the log message
        match record.level() {
            log::Level::Error => {
                writer.set_fg_color(Color::Red);
                let _ = write!(writer, " [ERROR] ");
                writer.reset_colors();
            }
            log::Level::Warn => {
                writer.set_fg_color(Color::Yellow);
                let _ = write!(writer, " [WARN] ");
                writer.reset_colors();
            }
            log::Level::Info => {
                writer.set_fg_color(Color::Blue);
                let _ = write!(writer, " [INFO] ");
                writer.reset_colors();
            }
            log::Level::Debug => {
                writer.set_fg_color(Color::Cyan);
                let _ = write!(writer, " [DEBUG] ");
                writer.reset_colors();
            }
            log::Level::Trace => {
                let _ = write!(writer, " [TRACE] ");
            }
        }

        if record.level() >= Level::Debug {
            let current_thread = std::thread::current();

            // `TreadId::as_u64()` is still unstable, so we'll work around this parsing the `Debug`
            // representation
            let id = format!("{:?}", current_thread.id());
            let id = id
                .strip_prefix("ThreadId(")
                .and_then(|id| id.strip_suffix(')'))
                .unwrap_or(&id);

            let _ = match current_thread.name() {
                // Thread names can be useful for added context, but the default main thread doesn't
                // carry any special meaning and this can be deduced from the thread ID anyways
                Some(name) if name != "main" => write!(writer, "({id}, {name})"),
                _ => write!(writer, "({id})"),
            };

            if let Some(module_path) = record.module_path() {
                let _ = write!(writer, " {}", module_path);
            }

            let _ = write!(writer, ": ");
        } else if self.always_show_module_path {
            // The spacing is a bit different without a thread name, hence the else if here
            if let Some(module_path) = record.module_path() {
                let _ = write!(writer, "{}: ", module_path);
            }
        }

        if record.level() >= Level::Trace {
            let _ = match (record.file(), record.line()) {
                (Some(file), Some(line)) => write!(writer, "[{file}:{line}] "),
                (Some(file), None) => write!(writer, "[{file}] "),
                _ => Ok(()),
            };
        }

        let _ = writeln!(writer, "{}", record.args());

        // Every line should be flushed immediately to avoid surprises
        let _ = writer.flush();
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.max_log_level && !self.target_enabled(metadata.target())
    }

    fn log(&self, record: &log::Record) {
        if !self.target_enabled(
            record
                .module_path()
                .unwrap_or_else(|| record.metadata().target()),
        ) {
            return;
        }

        // See the bullet in the repo's readme. Super specific situations call for super specific
        // solutions. `assert_no_alloc` with the log feature enabled may cause an allocation that
        // occurs while logging to be logged. In that case `self.output_target.lock()` would
        // deadlock. To still allowing getting this log output to the correct location in accordance
        // with the `NIH_LOG` environment variable we'll explicitly detect reentrant logging calls
        // since this won't occur in any other situation.q
        IS_REENTRANT_LOGGING_CALL.with(|is_reentrant_logging_call| {
            if is_reentrant_logging_call.get() {
                // This will also allocate, but `assert_no_alloc` allows allocations in its
                // allocation failure handler
                let mut target = OutputTargetImpl::default_from_environment();
                self.do_log(target.writer(), record);
            } else {
                is_reentrant_logging_call.set(true);

                // We currently don't catch panics here because of the assumption that any panics
                // raised are allocation failures from `assert_no_alloc`, and we already reserve
                // quite a bit of capacity to prevent additional allocations (though this as a whole
                // of course still isn't realtime-safe)
                let mut target = match self.output_target.lock() {
                    Ok(target) => target,
                    Err(err) => err.into_inner(),
                };
                self.do_log(target.writer(), record);

                is_reentrant_logging_call.set(false);
            }
        });
    }

    fn flush(&self) {
        let _ = self
            .output_target
            .lock()
            .expect("Mutex poisoned")
            .writer()
            .flush();
    }
}
