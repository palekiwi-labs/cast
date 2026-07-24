{
  description = "cast global harness devShells";

  # nixConfig declares the numtide binary cache so prebuilt harness packages
  # are fetched rather than built from source. `cast`'s dev container enables
  # `accept-flake-config = true`, so these are honoured non-interactively.
  nixConfig = {
    extra-substituters = [ "https://cache.numtide.com" ];
    extra-trusted-public-keys = [
      "cache.numtide.com-1:2ps1kLBUWjxIneOy1Ik6cQjb41X0iXVXeHigGmycPPE="
    ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    llm-agents = {
      url = "github:numtide/llm-agents.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, llm-agents, ... }:
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
