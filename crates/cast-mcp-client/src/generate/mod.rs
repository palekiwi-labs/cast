mod params;
mod script;

pub(crate) use params::camel_to_kebab;
pub use script::generate_script;
pub(crate) use script::now_unix;
