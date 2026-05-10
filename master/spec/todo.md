# MASTER TODO

## Features

## Security

- [ ] `cast config allow` should overwrite, not append. We want to maintain
  exactly 1 approved version of config per workspace. This means we could
  simplify `cast config deny` to simply remove but I don't think we should
  do that because it is better to assume other entries exist

### Subcommands

- [ ] `nix-daemon flake`
  - [ ] `nix-daemon flake init`
  - [ ] `nix-daemon flake update`

- [ ] utilities
  - [ ] `ps`: lists all running `cast` containers
  - [ ] `stop <agent>`: allows force-stopping a container

## Config

- [ ] `nix_warn_dirty` default: false

- [ ] Examine if we can drop special handling of `opencode_config` and `opencode_config_dir`
  Currently, these point to filepaths and their values get added to the volume mounts.
  Investigate if this can be dropped in favor of users having to explicitly define
  extra data mounts in the cast.json config. This will simplify our module logic,
  keep it more transparent and leverage the extra data mouns feature.
