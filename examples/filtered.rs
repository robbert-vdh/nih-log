use log::LevelFilter;

fn main() {
    nih_log::LoggerBuilder::new(LevelFilter::Trace)
        // Filtering only works with exact matches, so the log messages from
        // `some_module::some_sub_module` will still show up
        .filter_module("filtered::some_module")
        .build_global()
        .expect("A logger has already been set up");

    some_module::log();
    some_module::some_sub_module::log();
}

mod some_module {
    pub fn log() {
        log::debug!("This message is filtered out");
    }

    pub mod some_sub_module {
        pub fn log() {
            log::debug!(
                "This message is still printed because the module filtering uses exact matches"
            );
        }
    }
}
