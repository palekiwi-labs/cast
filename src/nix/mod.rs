pub mod config;
pub mod daemon;
mod dev;
pub mod dev_image;
mod extra_dirs;
mod image;
mod image_hash;

#[derive(Debug, Clone, Copy, Default)]
pub struct BuildOptions {
    pub force: bool,
    pub no_cache: bool,
}

pub use daemon::{build, ensure_running, stop};
pub use dev::build_dev;
