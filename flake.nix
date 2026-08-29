{
  description = "cast - coding agent sandbox tool";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, fenix, flake-utils, ... }:
    {
      # Global-flake template for `cast`'s harness devShells. Consumed via
      # `nix flake init -t github:palekiwi-labs/cast#global`. The same content
      # is embedded in the binary (crates/cast/assets/global-flake-template/flake.nix) and
      # auto-scaffolded on first `cast run` when no global flake is present.
      templates.global = {
        path = ./crates/cast/assets/global-flake-template;
        description = "cast global harness devShells (opencode, pi, claudecode, universal)";
      };
      templates.default = self.templates.global;
    } //
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        rustToolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-P30Tm3O7vQAE725YtDCDHGjNrSsfZO4us11UwJGZSJo=";
        };
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
        common = {
          version = "0.3.0-herdr-spike";
          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = path: type:
              let
                relPath = pkgs.lib.removePrefix (toString ./.) (toString path);
              in
              (pkgs.lib.hasPrefix "/crates/cast/docs" relPath) ||
              (pkgs.lib.cleanSourceFilter path type);
          };
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = [ rustToolchain pkgs.cacert ];
          SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
        };
      in
      {
        packages = {
          cast = rustPlatform.buildRustPackage (common // {
            pname = "cast";
            cargoBuildFlags = [ "-p" "cast" ];
            cargoTestFlags = [ "-p" "cast" ];
            nativeCheckInputs = [ pkgs.git ];
            meta = with pkgs.lib; {
              description = "cast - coding agent sandbox tool";
              homepage = "https://github.com/palekiwi-labs/cast";
              license = licenses.mit;
            };
          });

          cast-mcp-client = rustPlatform.buildRustPackage (common // {
            pname = "cast-mcp-client";
            cargoBuildFlags = [ "-p" "cast-mcp-client" ];
            cargoTestFlags = [ "-p" "cast-mcp-client" ];
            nativeCheckInputs = [ pkgs.bash pkgs.jq ];
            meta = with pkgs.lib; {
              description = "Lightweight MCP client for cast";
              homepage = "https://github.com/palekiwi-labs/cast";
              license = licenses.mit;
            };
          });

          default = self.packages.${system}.cast;
        };

        devShells.default = pkgs.mkShell
          {
            name = "cast";
            buildInputs = [
              rustToolchain
              pkgs.cargo-expand
              pkgs.cargo-watch
              pkgs.cargo-edit
            ];

            shellHook = ''
              echo "Rust development environment ready!" >&2
              echo "Rust version: $(rustc --version)" >&2
            '';
          };
      });
}
