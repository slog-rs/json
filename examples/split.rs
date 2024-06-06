//! Example of logging to different files, filtering by target
//! - NDJSON format
//! - with timestamp on rfc3339

#[macro_use]
extern crate log;
pub use example_log::create;
pub mod example_log {
    use slog::{Drain, Filter, FnValue, PushFnValue, Record};

    pub struct ExampleGuard {
        guard: slog_scope::GlobalLoggerGuard,
    }
    impl ExampleGuard {
        pub fn grub(self) {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            self.guard.cancel_reset();
        }
    }

    pub fn create() -> ExampleGuard {
        let general_drain = {
            let file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .truncate(false)
                .open("common.log")
                .unwrap();
            let builder = slog_json::Json::new(file).set_newlines(true);

            let drain_json = builder.build().fuse();
            let drain_async = slog_async::Async::new(drain_json).build().fuse();
            let drain_env = slog_envlogger::new(drain_async).fuse();
            let drain_filter = Filter::new(drain_env, |record| {
                !record.tag().starts_with("YOUR_KEY")
            })
            .fuse();
            drain_filter
        };

        let event_drain = {
            let file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .truncate(false)
                .open("event.log")
                .unwrap();
            let builder = slog_json::Json::new(file).set_newlines(true);

            let drain_json = builder.build().fuse();
            let drain_async: slog::Fuse<slog_async::Async> =
                slog_async::Async::new(drain_json).build().fuse();
            let drain_env = slog_envlogger::new(drain_async).fuse();
            let drain_filter = Filter::new(drain_env, |record| {
                record.tag().starts_with("YOUR_KEY")
            })
            .fuse();
            drain_filter
        };

        let drain = slog::Duplicate::new(general_drain, event_drain).fuse();

        let values = slog::o!(
            "v" => env!("CARGO_PKG_VERSION"),
            "OS" => std::env::consts::OS,
            "t" => PushFnValue(move |_: &Record, ser| {
                ser.emit(chrono::Utc::now().to_rfc3339())
            }),
            "l" => FnValue(move |rinfo: &Record| {
                rinfo.level().as_str()
            }),
            "m" => PushFnValue(move |record: &Record, ser| {
                ser.emit(record.msg())
            }),
        );

        let logger = slog::Logger::root(drain, values);
        let guard = slog_scope::set_global_logger(logger);

        slog_stdlog::init().unwrap();

        ExampleGuard { guard }
    }
}

fn main() {
    std::env::set_var("RUST_LOG", "debug");
    let example_guard_log = example_log::create();

    debug!("message");

    warn!("message");

    info!("message: {}", "value");

    info!(target: "YOUR_KEY ...", "message");

    error!("message");

    trace!("message: {:?}", ["message", "message", "message"]);

    // The logs should have time to burn to disk
    example_guard_log.grub();
}
