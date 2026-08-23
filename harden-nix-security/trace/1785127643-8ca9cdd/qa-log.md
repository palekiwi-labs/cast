## entering the global devshell in a container

```bash
harden-nix-security  feat/harden-nix-security [󱄅cast-env] 
󰲒 cargo run -p cast -- exec --raw opencode /bin/bash
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
     Running `target/debug/cast exec --raw opencode /bin/bash`
dev image already exists: localhost/cast:0.2.0
pl@1f0a64bc8d73:~/code/palekiwi-labs/cast/.worktrees/harden-nix-security$ nix develop ~/.config/cast/nix 
warning: ignoring untrusted substituter 'https://cache.nixos.org/', you are not a trusted user.
Run `man nix.conf` for more information on the `substituters` configuration option.
warning: ignoring the client-specified setting 'trusted-public-keys', because it is a restricted setting and you are not a trusted user
warning: ignoring untrusted substituter 'https://cache.nixos.org/', you are not a trusted user.
Run `man nix.conf` for more information on the `substituters` configuration option.
warning: ignoring the client-specified setting 'trusted-public-keys', because it is a restricted setting and you are not a trusted user
CAST Global Nix Environment Loaded
```
