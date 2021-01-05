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
use std::ops;
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone)]
pub struct Temp {
    path: PathBuf,
}

fn create_path() -> PathBuf {
    create_path_in(env::temp_dir())
}

fn create_path_in(path: PathBuf) -> PathBuf {
    let mut path = path;
    let dir_uuid = Uuid::new_v4();

    path.push(dir_uuid.to_simple().to_string());
    path
}

impl Temp {
    /// Create a temporary directory.
    pub fn new_dir() -> io::Result<Self> {
        let path = create_path();
        Self::create_dir(&path)?;

        let temp = Temp { path: path };

        Ok(temp)
    }

    /// Create a new temporary directory in an existing directory
    pub fn new_dir_in<P: AsRef<Path>>(directory: P) -> io::Result<Self> {
        let path = create_path_in(directory.as_ref().to_path_buf());
        Self::create_dir(&path)?;

        let temp = Temp { path: path };

        Ok(temp)
    }

    /// Create a new temporary file in an existing directory
    pub fn new_file_in<P: AsRef<Path>>(directory: P) -> io::Result<Self> {
        let path = create_path_in(directory.as_ref().to_path_buf());
        Self::create_file(&path)?;

        let temp = Temp { path: path };

        Ok(temp)
    }

    /// Create a temporary file.
    pub fn new_file() -> io::Result<Self> {
        let path = create_path();
        Self::create_file(&path)?;

        let temp = Temp { path: path };

        Ok(temp)
    }

    /// Create new uninitialized temporary path, i.e. a file or directory isn't created automatically
    pub fn new_path() -> Self {
        let path = create_path();

        Temp { path }
    }

    /// Create a new uninitialized temporary path in an existing directory i.e. a file or directory
    /// isn't created automatically
    pub fn new_path_in<P: AsRef<Path>>(directory: P) -> Self {
        let path = create_path_in(directory.as_ref().to_path_buf());

        Temp { path }
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
    pub fn release(self) -> PathBuf {
        use std::mem::{forget, transmute_copy};

        let path = unsafe { transmute_copy(&self.path) };
        forget(self);
        path
    }

    fn create_file(path: &Path) -> io::Result<()> {
        let mut builder = fs::OpenOptions::new();
        builder.write(true).create_new(true);

        #[cfg(unix)]
        builder.mode(0o600);

        builder.open(path)?;
        Ok(())
    }

    fn create_dir(path: &Path) -> io::Result<()> {
        let mut builder = fs::DirBuilder::new();

        #[cfg(unix)]
        builder.mode(0o700);

        builder.create(path)
    }
}

impl AsRef<Path> for Temp {
    fn as_ref(&self) -> &Path {
        &self.path.as_path()
    }
}

impl ops::Deref for Temp {
    type Target = PathBuf;
    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl ops::DerefMut for Temp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.path
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        // Drop is blocking (make non-blocking?)
        if let Err(e) = if self.path.is_dir() {
            fs::remove_dir_all(&self)
        } else {
            fs::remove_file(&self)
        } {
            if ::std::thread::panicking() {
                eprintln!("Could not remove path {:?}: {}", self.path, e);
            } else {
                panic!("Could not remove path {:?}: {}", self.path, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;

    #[test]
    fn it_should_create_file_in_dir() {
        let in_dir;
        {
            let temp_dir = Temp::new_dir().unwrap();

            in_dir = temp_dir.path.clone();

            {
                let temp_file = Temp::new_file_in(in_dir).unwrap();
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
            let temp_file = Temp::new_file().unwrap();
            path_buf = temp_file.release();
        }
        assert!(path_buf.exists());
        fs::remove_file(path_buf).unwrap();
    }

    #[test]
    fn it_should_not_drop_released_dir() {
        let path_buf;
        {
            let temp_dir = Temp::new_dir().unwrap();
            path_buf = temp_dir.release();
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

    #[test]
    fn uninitialized_panic_on_drop() {
        use std::panic::catch_unwind;

        assert!(catch_unwind(|| {
            let _ = Temp::new_path();
        })
        .is_err());
    }

    #[test]
    fn uninitialized_file() {
        let temp = Temp::new_path();
        assert!(!temp.exists());
        let _file = File::create(&temp);
        assert!(temp.exists());
    }

    #[test]
    fn uninitialized_no_panic_on_drop_with_release() {
        let t = Temp::new_path();
        t.release();
    }

    #[test]
    #[cfg(unix)]
    fn unix_socket() {
        let t = Temp::new_path();
        println!("Path is {:?}", t.to_str());
        let socket = std::os::unix::net::UnixListener::bind(t.to_str().unwrap());
        drop(socket);
        drop(t);
    }
}
