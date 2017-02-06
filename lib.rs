//! Json formatter for `slog-rs`
//!
//! ```
//! #[macro_use]
//! extern crate slog;
//! extern crate slog_json;
//! extern crate slog_stream;
//!
//! use slog::DrainExt;
//!
//! fn main() {
//!     let root = slog::Logger::root(
//!         slog_stream::stream(
//!             std::io::stderr(),
//!             slog_json::default()
//!         ).fuse(),
//!         o!("build-id" => "8dfljdf")
//!     );
//! }
//! ```
#![warn(missing_docs)]

extern crate slog;
extern crate slog_serde;
extern crate slog_stream;
extern crate serde_json;
extern crate chrono;

use std::io;

use slog_serde::SerdeSerializer;
use slog::Record;
use slog::{Level, OwnedKVList, KV, SingleKV};
use slog::Level::*;
use slog::{FnValue, PushFnValue};

fn level_to_string(level: Level) -> &'static str {
    match level {
        Critical => "CRIT",
        Error => "ERRO",
        Warning => "WARN",
        Info => "INFO",
        Debug => "DEBG",
        Trace => "TRCE",
    }
}

/// Json formatter
///
/// Each record will be printed as a Json map.
pub struct Format {
    newlines: bool,
    values: Vec<Box<KV+'static+Send+Sync>>,
}

impl Format {
    /// Build a Json formatter
    pub fn new() -> FormatBuilder {
        FormatBuilder::new()
    }
}

/// Json formatter builder
///
/// Create with `Format::build`.
pub struct FormatBuilder {
    newlines: bool,
    values: Vec<Box<KV+'static+Send+Sync>>,
}

impl FormatBuilder {
    fn new() -> Self {
        FormatBuilder {
            newlines: true,
            values: vec!(),
        }
    }

    /// Build `Json` format
    ///
    /// This consumes the builder.
    pub fn build(self) -> Format {
        Format {
            values: self.values,
            newlines: self.newlines,
        }
    }

    /// Set writing a newline after ever log record
    pub fn set_newlines(mut self, enabled: bool) -> Self {
        self.newlines = enabled;
        self
    }

    /// Add custom values to be printed with this formatter
    pub fn add_key_values(mut self, mut values: Vec<Box<KV+'static+Send+Sync>>) -> Self {
        for kv in values.drain(..) {
            self = self.add_key_value(kv);
        }
        self
    }

    /// Add custom values to be printed with this formatter
    pub fn add_key_value(mut self, value: Box<KV+'static+Send+Sync>) -> Self {
        self.values.push(value);
        self
    }

    /// Add default key-values:
    /// * `ts` - timestamp
    /// * `level` - record logging level name
    /// * `msg` - msg - formatted logging message
    pub fn add_default_keys(self) -> Self {
        self.add_key_values(
            vec![
                Box::new(SingleKV("ts", PushFnValue(move |_ : &Record, ser| {
                    ser.serialize(chrono::Local::now().to_rfc3339())
                }))),

                Box::new(SingleKV("level", FnValue(move |rinfo : &Record| {
                    level_to_string(rinfo.level())
                }))),
                Box::new(SingleKV("msg", PushFnValue(move |record : &Record, ser| {
                    ser.serialize(record.msg())
                })))

            ],
            )
    }
}

impl slog_stream::Format for Format {
    fn format(&self,
              io: &mut io::Write,
              rinfo: &Record,
              logger_values: &OwnedKVList)
              -> io::Result<()> {

        let io = {
            let serializer = serde_json::Serializer::new(io);
            let mut serializer = try!(SerdeSerializer::start(serializer, None));

            for kv in self.values.iter() {
                try!(kv.serialize(rinfo, &mut serializer));
            }

            for kv in logger_values.iter_groups() {
                try!(kv.serialize(rinfo, &mut serializer));
            }

            for kv in rinfo.values().iter() {
                try!(kv.serialize(rinfo, &mut serializer));
            }
            let (serializer, res) = serializer.end();

            let _ = try!(res);
            serializer.into_inner()
        };
        if self.newlines {
            let _ = try!(io.write_all("\n".as_bytes()));
        }
        Ok(())
    }
}

/// Create new `FormatBuilder` to create `Format`
pub fn new() -> FormatBuilder {
    Format::new()
}

/// Default json `Format`
pub fn default() -> Format {
    Format::new().add_default_keys().build()
}
