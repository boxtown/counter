use std::fs::{File, OpenOptions};
use super::{Result, Error};

pub fn read_file(path: &str) -> Result<()> {
    let file = try!(OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(Error::IO));

    Ok(())
}