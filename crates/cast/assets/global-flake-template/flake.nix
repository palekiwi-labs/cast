{
  description = "cast global harness devShells";

  # Binary caches are deliberately NOT declared here.
  #
  # Inside `cast`'s dev container the nix daemon runs with
  # `trusted-users = root`, so the dev user is non-trusted and flake-declared
  # substituters/keys are rejected by the daemon. A live `nixConfig` block is
  # worse than useless there: nix prompts for approval on every devshell entry,
  # and the prompt cannot be usefully answered -- the settings are rejected
  # whichever way it is answered. `cast` provisions caches daemon-side instead,
  # from `~/.config/cast/cast.json` (seeded with the numtide cache on first
  # run); edit that file to add or remove caches.
  #
  # If you use this flake OUTSIDE `cast`, in a standalone or trusted nix
  # environment, uncomment the block below so prebuilt harness packages are
  # fetched from the numtide cache rather than built from source:
  #
  #   nixConfig = {
  #     extra-substituters = [ "https://cache.numtide.com" ];
  #     extra-trusted-public-keys = [
  #       "niks3.numtide.com-1:DTx8wZduET09hRmMtKdQDxNNthLQETkc/yaX7M4qK0g="
  #     ];
  #   };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    llm-agents = {
      url = "github:numtide/llm-agents.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, flake-utils, llm-agents, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        agents = llm-agents.packages.${system};

        # Shell-composition reference pattern: shared base inputs are extended
        # per shell. Add tools every harness should see to `baseInputs`; each
        # named shell layers its own harness on top.
        baseInputs = [ pkgs.git pkgs.ripgrep ];

        mkShell = name: extra: pkgs.mkShell {
          inherit name;
          buildInputs = baseInputs ++ extra;
          shellHook = ''echo "cast: entered '${name}' devShell" >&2'';
        };

        opencode = agents.opencode;
        pi = agents.pi;
        claudecode = agents.claude-code;
      in
      {
        devShells = {
          # `default` provides the shared base tooling only (no harness).
          default = mkShell "default" [ ];

          # Per-harness shells (selected by `cast run <agent>`).
          opencode = mkShell "opencode" [ opencode ];
          pi = mkShell "pi" [ pi ];
          claudecode = mkShell "claudecode" [ claudecode ];

          # Universal shell exposing every harness in one environment.
          universal = mkShell "universal" [ opencode pi claudecode ];
        };
      });
}
