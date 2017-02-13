thread_local! {
    static TL_BUF: RefCell<String> = RefCell::new(String::with_capacity(128))
}

/// `slog::Serializer` adapter for `serde::Serializer`
///
/// Newtype to wrap serde Serializer, so that `Serialize` can be implemented
/// for it
struct SerdeSerializer<S: serde::Serializer>{
    /// Current state of map serializing: `serde::Seriaizer::MapState`
    ser_map : S::SerializeMap,
}

impl<S: serde::Serializer> SerdeSerializer<S> {

    /// Start serializing map of values
    fn start(ser : S, len: Option<usize>) -> result::Result<Self, slog::Error> {
        let ser_map = try!(
            ser.serialize_map(len)
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "serde serialization error"))
        );
        Ok(SerdeSerializer {
            ser_map: ser_map,
        })
    }

    /// Finish serialization, and return the serializer
    fn end(self) -> std::result::Result<S::Ok, S::Error> {
        let res = self.ser_map.end();
        res
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
    fn emit_arguments(&mut self, key: &str, val: &fmt::Arguments) -> slog::Result {

        TL_BUF.with(|buf| {
            let mut buf = buf.borrow_mut();

            buf.write_fmt(*val).unwrap();

            let res = {
                || {
                    impl_m!(self, key, &*buf)
                }
            }();
            buf.clear();
            res
        })
    }
}
