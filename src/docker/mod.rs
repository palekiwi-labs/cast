pub mod args;
pub mod client;
pub mod image_hash;

#[derive(Debug, Clone, Copy, Default)]
pub struct BuildOptions {
    pub force: bool,
    pub no_cache: bool,
}
