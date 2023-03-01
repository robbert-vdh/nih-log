use log::LevelFilter;

fn main() {
    // The log output is determined by the `NIH_LOG` environment variable as dedicated by the rules
    // outlined in the repository's readme
    nih_log::LoggerBuilder::new(LevelFilter::Trace)
        .build_global()
        // In this example something would be have gone very wrong if we cannot set up the logger.
        // If there however is a possibility that the logger is configured multiple times then this
        // error should be handled appropriately.
        .expect("A logger has already been set up");

    // When changing some of the level filter above some of these messages may no longer be printed
    log::error!("This is an error");
    log::warn!("This is a warning");
    log::info!("This is a regular log message");
    log::debug!("This is a debug message, usually only made visible during debug builds");
    log::trace!("This is a trace message, usually hidden unless explicitly opted into");

    // Debug and trace messages contain the module path
    some_module::log_from_module();
}

mod some_module {
    pub fn log_from_module() {
        log::debug!("This is a debug message printed from another module");
    }
}
