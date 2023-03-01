# NIH-log

A logger for the [log](https://crates.io/crates/log) crate made specifically to
cater to the needs of the [NIH-plug](https://github.com/robbert-vdh/nih-plug)
plugin framework.

## Features

- Log messages are formatted similarly to
  [simplelog](https://crates.io/crates/simplelog). Because simplelog is great.
- The log's output target can be changed by setting the `NIH_LOG` environment
  variable:

  - A value of `stderr` causes the log to be printed to STDERR.
  - _(not yet implemented)_ A value of `windbg` causes the log to be output to
    the Windows debugger using the
    [`OutputDebugString()`](https://learn.microsoft.com/en-us/windows/win32/api/debugapi/nf-debugapi-outputdebugstringw)
    function.
  - Anything else is interpreted as a file name, which causes the log to be
    written to that file instead.

  The latter two options are useful on Windows where accessing the standard IO
  streams may be difficult.

- _(not yet implemented)_ If `NIH_LOG` is not set, then a dynamic logging output
  target is used instead. On Windows this causes log messages to be sent to the
  Windows debugger when one is attached. This check is done just before printing
  the message to make it possible to attach a debugger to a running process.
  When the debugger is not attached the output goes directly to STDERR. On
  non-Windows platforms STDERR is always used.
- _(not yet implemented)_ The logger's output can be changed to output to a custom function after the
  logger has been created. This makes it possible to integrate with external
  logging APIs that are not yet available when the logger is first initialized,
  like the CLAP plugin API's [logging
  extension](https://github.com/free-audio/clap/blob/main/include/clap/ext/log.h).
  - This API can be used multiple times, for instance by multiple CLAP plugin
    instances that have all received their own logger instance from the host. In
    that case all registered loggers are kept track of in a queue, and the first
    still active one is used.
  - If `NIH_LOG` was set explicitly, then this is honored and the regular
    behavior won't be overridden.
- The logger itself does not try to be realtime-safe. It does however avoid
  locking around the entire logging call so say if a panic happens while
  logging, that panic can also be logged using the logger.
