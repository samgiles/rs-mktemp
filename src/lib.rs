/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */
//! This module provides a simple way of creating temporary files and
//! directories where their lifetime is defined by the scope they exist in.
//!
//! Once the variable goes out of scope, the underlying file system resource is removed.
//!
//! # Examples
//!
//! ```
//! use mktemp::Temp;
//! use std::fs;
//!
//! {
//!   let temp_file = Temp::new_file().unwrap();
//!   assert!(fs::File::open(temp_file).is_ok());
//! }
//! // temp_file is cleaned from the fs here
//! ```
//!
extern crate uuid;

use std::env;
use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone)]
enum TempType {
    File,
    Dir,
}

#[derive(Clone)]
pub struct Temp {
    path: PathBuf,
    temp_type: TempType,
    released: bool,
}

fn create_path() -> PathBuf {
    create_path_in(env::temp_dir())
}

fn create_path_in(path: PathBuf) -> PathBuf {
    let mut path = path;
    let dir_uuid = Uuid::new_v4();

    path.push(dir_uuid.simple().to_string());
    path
}

impl Temp {
    /// Create a temporary directory.
    pub fn new_dir() -> io::Result<Self> {
        let path = create_path();
        Self::create_dir(&path)?;

        let temp = Temp {
            path: path,
            temp_type: TempType::Dir,
            released: false,
        };

        Ok(temp)
    }

    /// Create a new temporary directory in an existing directory
    pub fn new_dir_in(directory: &Path) -> io::Result<Self> {
        let path = create_path_in(directory.to_path_buf());
        Self::create_dir(&path)?;

        let temp = Temp {
            path: path,
            temp_type: TempType::Dir,
            released: false,
        };

        Ok(temp)
    }

    /// Create a new temporary file in an existing directory
    pub fn new_file_in(directory: &Path) -> io::Result<Self> {
        let path = create_path_in(directory.to_path_buf());
        Self::create_file(&path)?;

        let temp = Temp {
            path: path,
            temp_type: TempType::File,
            released: false,
        };

        Ok(temp)
    }

    /// Create a temporary file.
    pub fn new_file() -> io::Result<Self> {
        let path = create_path();
        Self::create_file(&path)?;

        let temp = Temp {
            path: path,
            temp_type: TempType::File,
            released: false,
        };

        Ok(temp)
    }

    /// Return this temporary file or directory as a PathBuf.
    ///
    /// # Examples
    ///
    /// ```
    /// use mktemp::Temp;
    ///
    /// let temp_dir = Temp::new_dir().unwrap();
    /// let mut path_buf = temp_dir.to_path_buf();
    /// ```
    pub fn to_path_buf(&self) -> PathBuf {
        PathBuf::from(&self.path)
    }

    /// Release ownership of the temporary file or directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use mktemp::Temp;
    /// let path_buf;
    /// {
    ///   let mut temp_dir = Temp::new_dir().unwrap();
    ///   path_buf = temp_dir.to_path_buf();
    ///   temp_dir.release();
    /// }
    /// assert!(path_buf.exists());
    /// ```
    pub fn release(&mut self) {
        self.released = true;
    }

    fn create_file(path: &Path) -> io::Result<()> {
        let mut builder = fs::OpenOptions::new();
        builder.write(true)
            .create_new(true);

        #[cfg(unix)]
        builder.mode(0o600);

        builder.open(path)?;
        Ok(())
    }

    fn remove_file(&self) -> io::Result<()> {
        fs::remove_file(self)
    }

    fn create_dir(path: &Path) -> io::Result<()> {
        let mut builder = fs::DirBuilder::new();

        #[cfg(unix)]
        builder.mode(0o700);

        builder.create(path)
    }

    fn remove_dir(&self) -> io::Result<()> {
        fs::remove_dir_all(self)
    }
}

impl AsRef<Path> for Temp {
    fn as_ref(&self) -> &Path {
        &self.path.as_path()
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        // Drop is blocking (make non-blocking?)
        if !self.released {
            let result = match self.temp_type {
                TempType::File => self.remove_file(),
                TempType::Dir => self.remove_dir(),
            };

            if let Err(e) = result {
                panic!("Could not remove path {:?}: {}", self.path, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;

    #[test]
    fn it_should_create_file_in_dir() {
        let in_dir;
        {
            let temp_dir = Temp::new_dir().unwrap();

            in_dir = temp_dir.path.clone();

            {
                let temp_file = Temp::new_file_in(in_dir.as_path()).unwrap();
                assert!(fs::metadata(temp_file).unwrap().is_file());
            }
        }
    }

    #[test]
    fn it_should_drop_file_out_of_scope() {
        let path;
        {
            let temp_file = Temp::new_file().unwrap();

            path = temp_file.path.clone();
            assert!(fs::metadata(temp_file).unwrap().is_file());
        }

        if let Err(e) = fs::metadata(path) {
            assert_eq!(e.kind(), io::ErrorKind::NotFound);
        } else {
            panic!("File was not removed");
        }
    }

    #[test]
    fn it_should_drop_dir_out_of_scope() {
        let path;
        {
            let temp_file = Temp::new_dir().unwrap();

            path = temp_file.path.clone();
            assert!(fs::metadata(temp_file).unwrap().is_dir());
        }

        if let Err(e) = fs::metadata(path) {
            assert_eq!(e.kind(), io::ErrorKind::NotFound);
        } else {
            panic!("File was not removed");
        }
    }

    #[test]
    fn it_should_not_drop_released_file() {
        let path_buf;
        {
            let mut temp_file = Temp::new_file().unwrap();
            path_buf = temp_file.to_path_buf();
            temp_file.release();
        }
        assert!(path_buf.exists());
        fs::remove_file(path_buf).unwrap();
    }

    #[test]
    fn it_should_not_drop_released_dir() {
        let path_buf;
        {
            let mut temp_dir = Temp::new_dir().unwrap();
            path_buf = temp_dir.to_path_buf();
            temp_dir.release();
        }
        assert!(path_buf.exists());
        fs::remove_dir_all(path_buf).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn temp_file_only_readable_by_owner() {
        let temp_file = Temp::new_file().unwrap();
        let mode = fs::metadata(temp_file.as_ref()).unwrap().mode();
        assert_eq!(0o600, mode & 0o777);
    }

    #[test]
    #[cfg(unix)]
    fn temp_dir_only_readable_by_owner() {
        let dir = Temp::new_dir().unwrap();
        let mode = fs::metadata(dir).unwrap().mode();
        assert_eq!(0o700, mode & 0o777)
    }

    #[test]
    fn target_dir_must_exist() {
        let temp_dir = Temp::new_dir().unwrap();
        let mut no_such_dir = temp_dir.as_ref().to_owned();
        no_such_dir.push("no_such_dir");

        match Temp::new_file_in(&no_such_dir) {
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => (),
            _ => panic!(),
        }

        match Temp::new_dir_in(&no_such_dir) {
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => (),
            _ => panic!(),
        }
    }
}
