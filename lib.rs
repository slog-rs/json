// {{{ Crate docs
//! JSON `Drain` for `slog-rs`
//!
//! ```
//! #[macro_use]
//! extern crate slog;
//! extern crate slog_json;
//!
//! use slog::Drain;
//! use std::sync::Mutex;
//!
//! fn main() {
//!     let root = slog::Logger::root(
//!         Mutex::new(slog_json::Json::default(std::io::stderr())).map(slog::Fuse),
//!         o!("version" => env!("CARGO_PKG_VERSION"))
//!     );
//! }
//! ```
// }}}

// {{{ Imports & meta
#![warn(missing_docs)]
#[macro_use]
extern crate slog;
extern crate chrono;
extern crate serde;
extern crate serde_json;

use serde::ser::SerializeMap;
use slog::{FnValue, PushFnValue};
use slog::{OwnedKVList, KV, SendSyncRefUnwindSafeKV};
use slog::Record;
use std::{io, result, fmt};

use std::cell::RefCell;
use std::fmt::Write;

// }}}

// {{{ Serialize
thread_local! {
    static TL_BUF: RefCell<String> = RefCell::new(String::with_capacity(128))
}

/// `slog::Serializer` adapter for `serde::Serializer`
///
/// Newtype to wrap serde Serializer, so that `Serialize` can be implemented
/// for it
struct SerdeSerializer<S: serde::Serializer> {
    /// Current state of map serializing: `serde::Serializer::MapState`
    ser_map: S::SerializeMap,
}

impl<S: serde::Serializer> SerdeSerializer<S> {
    /// Start serializing map of values
    fn start(ser: S, len: Option<usize>) -> result::Result<Self, slog::Error> {
        let ser_map = try!(ser.serialize_map(len)
            .map_err(|_| {
                io::Error::new(io::ErrorKind::Other,
                               "serde serialization error")
            }));
        Ok(SerdeSerializer { ser_map: ser_map })
    }

    /// Finish serialization, and return the serializer
    fn end(self) -> result::Result<S::Ok, S::Error> {
        self.ser_map.end()
    }
}

macro_rules! impl_m(
    ($s:expr, $key:expr, $val:expr) => ({
        try!($s.ser_map.serialize_entry($key, $val)
             .map_err(|_| io::Error::new(io::ErrorKind::Other, "serde serialization error")));
        Ok(())
    });
);

impl<S> slog::Serializer for SerdeSerializer<S>
    where S: serde::Serializer
{
    fn emit_bool(&mut self, key: &str, val: bool) -> slog::Result {
        impl_m!(self, key, &val)
    }

    fn emit_unit(&mut self, key: &str) -> slog::Result {
        impl_m!(self, key, &())
    }

    fn emit_char(&mut self, key: &str, val: char) -> slog::Result {
        impl_m!(self, key, &val)
    }

    fn emit_none(&mut self, key: &str) -> slog::Result {
        let val: Option<()> = None;
        impl_m!(self, key, &val)
    }
    fn emit_u8(&mut self, key: &str, val: u8) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_i8(&mut self, key: &str, val: i8) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_u16(&mut self, key: &str, val: u16) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_i16(&mut self, key: &str, val: i16) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_usize(&mut self, key: &str, val: usize) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_isize(&mut self, key: &str, val: isize) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_u32(&mut self, key: &str, val: u32) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_i32(&mut self, key: &str, val: i32) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_f32(&mut self, key: &str, val: f32) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_u64(&mut self, key: &str, val: u64) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_i64(&mut self, key: &str, val: i64) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_f64(&mut self, key: &str, val: f64) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_str(&mut self, key: &str, val: &str) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_arguments(&mut self,
                      key: &str,
                      val: &fmt::Arguments)
                      -> slog::Result {

        TL_BUF.with(|buf| {
            let mut buf = buf.borrow_mut();

            buf.write_fmt(*val).unwrap();

            let res = {
                || impl_m!(self, key, &*buf)
            }();
            buf.clear();
            res
        })
    }

    #[cfg(feature = "nested-values")]
    fn emit_serde(&mut self, key: &str, value: &slog::SerdeValue) -> slog::Result {
        impl_m!(self, key, value.as_serde())
    }
}
// }}}

// {{{ Json
/// Json `Drain`
///
/// Each record will be printed as a Json map
/// to a given `io`
pub struct Json<W: io::Write> {
    newlines: bool,
    values: Vec<OwnedKVList>,
    io: RefCell<W>,
}

impl<W> Json<W>
    where W: io::Write
{
    /// New `Json` `Drain` with default key-value pairs added
    pub fn default(io: W) -> Json<W> {
        JsonBuilder::new(io).add_default_keys().build()
    }

    /// Build custom `Json` `Drain`
    #[cfg_attr(feature = "cargo-clippy", allow(new_ret_no_self))]
    pub fn new(io: W) -> JsonBuilder<W> {
        JsonBuilder::new(io)
    }
}

// }}}

// {{{ JsonBuilder
/// Json `Drain` builder
///
/// Create with `Json::new`.
pub struct JsonBuilder<W: io::Write> {
    newlines: bool,
    values: Vec<OwnedKVList>,
    io: W,
}

impl<W> JsonBuilder<W>
    where W: io::Write
{
    fn new(io: W) -> Self {
        JsonBuilder {
            newlines: true,
            values: vec![],
            io: io,
        }
    }

    /// Build `Json` `Drain`
    ///
    /// This consumes the builder.
    pub fn build(self) -> Json<W> {
        Json {
            values: self.values,
            newlines: self.newlines,
            io: RefCell::new(self.io),
        }
    }

    /// Set writing a newline after every log record
    pub fn set_newlines(mut self, enabled: bool) -> Self {
        self.newlines = enabled;
        self
    }

    /// Add custom values to be printed with this formatter
    pub fn add_key_value<T>(mut self, value: slog::OwnedKV<T>) -> Self
        where T: SendSyncRefUnwindSafeKV + 'static
    {
        self.values.push(value.into());
        self
    }

    /// Add default key-values:
    ///
    /// * `ts` - timestamp
    /// * `level` - record logging level name
    /// * `msg` - msg - formatted logging message
    pub fn add_default_keys(self) -> Self {
        self.add_key_value(o!(
                "ts" => PushFnValue(move |_ : &Record, ser| {
                    ser.emit(chrono::Local::now().to_rfc3339())
                }),
                "level" => FnValue(move |rinfo : &Record| {
                    rinfo.level().as_short_str()
                }),
                "msg" => PushFnValue(move |record : &Record, ser| {
                    ser.emit(record.msg())
                }),
                ))
    }
}

impl<W> slog::Drain for Json<W>
    where W: io::Write
{
    type Ok = ();
    type Err = io::Error;
    fn log(&self,
           rinfo: &Record,
           logger_values: &OwnedKVList)
           -> io::Result<()> {

        let mut io = self.io.borrow_mut();
        let io = {
            let mut serializer = serde_json::Serializer::new(&mut *io);
            {
                let mut serializer =
                    try!(SerdeSerializer::start(&mut serializer, None));

                for kv in &self.values {
                    try!(kv.serialize(rinfo, &mut serializer));
                }

                try!(logger_values.serialize(rinfo, &mut serializer));

                try!(rinfo.kv().serialize(rinfo, &mut serializer));

                let res = serializer.end();

                try!(res.map_err(|e| io::Error::new(io::ErrorKind::Other, e)));
            }
            serializer.into_inner()
        };
        if self.newlines {
            try!(io.write_all("\n".as_bytes()));
        }
        Ok(())
    }
}
// }}}
// vim: foldmethod=marker foldmarker={{{,}}}
