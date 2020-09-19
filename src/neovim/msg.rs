use crate::color::Color;
use rmp::decode::{self as dec, DecodeStringError, NumValueReadError, ValueReadError};
use rmp::Marker;
use std::io::{self, Error, ErrorKind};
use std::panic::Location;

pub fn read_array_len(raw: &mut &[u8]) -> io::Result<usize> {
    Ok(dec::read_array_len(raw).map_err(value_read_error_to_io_error)? as usize)
}

pub fn read_map_len(raw: &mut &[u8]) -> io::Result<usize> {
    Ok(dec::read_map_len(raw).map_err(value_read_error_to_io_error)? as usize)
}

pub fn read_marker(raw: &mut &[u8]) -> io::Result<Marker> {
    dec::read_marker(raw).map_err(|e| e.0)
}

pub fn read_string<'a>(raw: &mut &'a [u8]) -> io::Result<&'a str> {
    let str_len = dec::read_str_len(raw).map_err(value_read_error_to_io_error)? as usize;
    let raw_buf = &raw[..str_len];
    *raw = &raw[str_len..];

    std::str::from_utf8(raw_buf).or_else(|_| err_invalid_input())
}

pub fn read_f64(raw: &mut &[u8]) -> io::Result<f64> {
    Ok(match read_marker(raw)? {
        Marker::F32 => dec::read_f32(raw).map_err(value_read_error_to_io_error)? as f64,
        Marker::F64 => dec::read_f64(raw).map_err(value_read_error_to_io_error)?,
        _ => return err_invalid_input(),
    })
}

pub fn read_u64(raw: &mut &[u8]) -> io::Result<u64> {
    dec::read_int(raw).map_err(num_value_read_error_to_io_error)
}

pub fn read_i64(raw: &mut &[u8]) -> io::Result<i64> {
    dec::read_int(raw).map_err(num_value_read_error_to_io_error)
}

pub fn read_bool(raw: &mut &[u8]) -> io::Result<bool> {
    dec::read_bool(raw).map_err(value_read_error_to_io_error)
}

pub fn read_color(raw: &mut &[u8]) -> io::Result<Color> {
    read_u64(raw).map(Color::from_rgb_u64)
}

pub fn read_ext_meta(raw: &mut &[u8]) -> io::Result<dec::ExtMeta> {
    dec::read_ext_meta(raw).map_err(value_read_error_to_io_error)
}

pub fn ensure_parameters_count(raw: &mut &[u8], count: usize) -> io::Result<()> {
    if read_array_len(raw)? == count {
        Ok(())
    } else {
        err_invalid_input()
    }
}

fn decode_string_error_to_io_error(err: DecodeStringError) -> Error {
    match err {
        DecodeStringError::InvalidDataRead(error) => error,
        DecodeStringError::InvalidMarkerRead(error) => error,
        err => Error::new(ErrorKind::InvalidInput, err.to_string()),
    }
}

fn value_read_error_to_io_error(err: ValueReadError) -> Error {
    match err {
        ValueReadError::InvalidDataRead(error) => error,
        ValueReadError::InvalidMarkerRead(error) => error,
        err => Error::new(ErrorKind::InvalidInput, err.to_string()),
    }
}

fn num_value_read_error_to_io_error(err: NumValueReadError) -> Error {
    match err {
        NumValueReadError::InvalidDataRead(error) => error,
        NumValueReadError::InvalidMarkerRead(error) => error,
        err => Error::new(ErrorKind::InvalidInput, err.to_string()),
    }
}

#[track_caller]
pub fn err_invalid_input<T>() -> io::Result<T> {
    log::error!("invalid message pack input received at {}", Location::caller());
    Err(Error::new(
        ErrorKind::InvalidInput,
        "expected RPC notification with one argument",
    ))
}

#[track_caller]
pub fn err_invalid_method<T>() -> io::Result<T> {
    log::error!("invalid message pack method received at {}", Location::caller());
    Err(Error::new(
        ErrorKind::InvalidInput,
        "expected RPC notification with one argument, found different method",
    ))
}
