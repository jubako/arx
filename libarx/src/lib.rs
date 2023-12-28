//#![feature(get_mut_unchecked)]

mod arx;
#[cfg(not(windows))]
mod arx_fs;
#[cfg(feature = "cmd_utils")]
pub mod cmd_utils;
mod common;
pub mod create;
mod entry;
mod tools;
pub mod walk;

pub use arx::Arx;
#[cfg(not(windows))]
pub use arx_fs::{ArxFs, Stats};
pub use common::{
    AllProperties, Builder, Entry, FullBuilderTrait, Path, PathBuf, Reader, VENDOR_ID,
};
pub use entry::*;
pub use tools::extract;
pub use walk::*;
