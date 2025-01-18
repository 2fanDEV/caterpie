use std::{fs, path::Path};

use log::error;

pub fn read_file<P: AsRef<Path> + std::fmt::Debug + ToString>(path: P) -> Result<Vec<u8>, &'static str>
{
    let file = fs::read(path.to_string());
    match file {
        Ok(file_contents) => Ok(file_contents),
        Err(error_msg) => {
            error!(
                "Failed to read the contents of path {:?}, with following error message: '{:?}'",
                path, error_msg
            );
            Err("Failed to read file")
        }
    }
}
