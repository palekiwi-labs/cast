use crate::config::Config;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::user::ResolvedUser;
use anyhow::Result;

const IMAGE_BASE: &str = "localhost/cast";
const OCX_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get the full image tag: `localhost/cast:{ocx_version}-pi-{pi_version}`
pub fn get_image_tag(pi_version: &str) -> String {
    format!("{}:{}-pi-{}", IMAGE_BASE, OCX_VERSION, pi_version)
}

pub fn ensure_dev_image(
    _docker: &DockerClient,
    _config: &Config,
    _user: &ResolvedUser,
    _version: &str,
    _opts: BuildOptions,
) -> Result<()> {
    // Implementation for building the image will come in Slice 4
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_image_tag_format() {
        assert_eq!(
            get_image_tag("v0.71.0"),
            format!("localhost/cast:{}-pi-v0.71.0", env!("CARGO_PKG_VERSION"))
        );
    }
}
