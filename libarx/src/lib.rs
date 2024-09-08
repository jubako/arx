//#![feature(get_mut_unchecked)]

mod arx;
#[cfg(all(not(windows), feature = "fuse"))]
mod arx_fs;
#[cfg(feature = "cmd_utils")]
pub mod cmd_utils;
mod common;
pub mod create;
mod entry;
mod tools;
pub mod walk;

pub use arx::Arx;
#[cfg(all(not(windows), feature = "fuse"))]
pub use arx_fs::{ArxFs, Stats};
pub use common::{
    AllProperties, Builder, Entry, FromPathError, FromPathErrorKind, FullBuilderTrait, Path,
    PathBuf, VENDOR_ID,
};
pub use entry::*;
pub use tools::{extract, extract_arx, extract_arx_range};
pub use walk::*;
