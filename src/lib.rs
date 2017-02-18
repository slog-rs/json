//! Json formatter for `slog-rs`
//!
//! ```
//! #[macro_use]
//! extern crate slog;
//! extern crate slog_json;
//!
//! use slog::DrainExt;
//! use std::sync::Mutex;
//!
//! fn main() {
//!     let root = slog::Logger::root(
//!             Mutex::new(slog_json::default(std::io::stderr())).fuse(),
//!         o!("build-id" => "8dfljdf")
//!     );
//! }
//! ```
#![warn(missing_docs)]

#[macro_use]
extern crate slog;
extern crate chrono;
extern crate serde;
extern crate serde_json;

use std::{io, result, fmt};

use std::cell::RefCell;
use std::fmt::Write;
use slog::Record;
use slog::{Level, OwnedKVList, KV};
use slog::Level::*;
use slog::{FnValue, PushFnValue};
use serde::ser::SerializeMap;

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

include!("_ser.rs");

/// Json writter
///
/// Each record will be printed as a Json map
/// to a given `io`
pub struct Json<W : io::Write> {
    newlines: bool,
    values: Vec<OwnedKVList>,
    io : RefCell<W>,
}

impl<W> Json<W>
where W : io::Write {
    /// Build a Json formatter
    pub fn new(io : W) -> JsonBuilder<W> {
        JsonBuilder::new(io)
    }
}

/// Json formatter builder
///
/// Create with `Json::build`.
pub struct JsonBuilder<W : io::Write> {
    newlines: bool,
    values: Vec<OwnedKVList>,
    io : W,
}

impl<W> JsonBuilder<W>
where W : io::Write {
    fn new(io : W) -> Self {
        JsonBuilder {
            newlines: true,
            values: vec!(),
            io : io,
        }
    }

    /// Build `Json` format
    ///
    /// This consumes the builder.
    pub fn build(self) -> Json<W> {
        Json {
            values: self.values,
            newlines: self.newlines,
            io: RefCell::new(self.io),
        }
    }

    /// Set writing a newline after ever log record
    pub fn set_newlines(mut self, enabled: bool) -> Self {
        self.newlines = enabled;
        self
    }

    /// Add custom values to be printed with this formatter
    pub fn add_key_value<T>(mut self, value: slog::OwnedKV<T>) -> Self
    where T : KV + Send + Sync + 'static {
        self.values.push(value.into());
        self
    }

    /// Add default key-values:
    /// * `ts` - timestamp
    /// * `level` - record logging level name
    /// * `msg` - msg - formatted logging message
    pub fn add_default_keys(self) -> Self {
        self.add_key_value(
            o!(
                "ts" => PushFnValue(move |_ : &Record, ser| {
                    ser.serialize(chrono::Local::now().to_rfc3339())
                }),
                "level" => FnValue(move |rinfo : &Record| {
                    level_to_string(rinfo.level())
                }),
                "msg" => PushFnValue(move |record : &Record, ser| {
                    ser.serialize(record.msg())
                }),
                )
            )
    }
}

impl<W> slog::Drain for Json<W>
where W : io::Write {
    type Ok = ();
    type Err = io::Error;
    fn log(&self,
              rinfo: &Record,
              logger_values: &OwnedKVList)
              -> io::Result<()> {

        let mut io = self.io.borrow_mut();
        let mut io = {
            let mut serializer = serde_json::Serializer::new(&mut *io);
            {
                let mut serializer = try!(SerdeSerializer::start(&mut serializer, None));

                for kv in self.values.iter() {
                    try!(kv.serialize(rinfo, &mut serializer));
                }

                try!(logger_values.serialize(rinfo, &mut serializer));

                try!(rinfo.kv().serialize(rinfo, &mut serializer));

                let res = serializer.end();

                let _ = try!(res.map_err(|e| io::Error::new(io::ErrorKind::Other, e)));
            }
            serializer.into_inner()
        };
        if self.newlines {
            let _ = try!(io.write_all("\n".as_bytes()));
        }
        Ok(())
    }
}

/// Create new `JsonBuilder` to create `Json`
pub fn custom<W : io::Write>(io : W) -> JsonBuilder<W> {
    Json::new(io)
}

/// Default json `Json`
pub fn default<W : io::Write>(io : W) -> Json<W> {
    Json::new(io).add_default_keys().build()
}
