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
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        rustToolchain = fenix.packages.${system}.stable.toolchain;
        common = {
          version = "0.1.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = [ rustToolchain pkgs.cacert ];
          SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
        };
      in
      {
        packages = {
          cast = pkgs.rustPlatform.buildRustPackage (common // {
            pname = "cast";
            cargoBuildFlags = [ "-p" "cast" ];
            meta = with pkgs.lib; {
              description = "cast - coding agent sandbox tool";
              homepage = "https://github.com/palekiwi-labs/cast";
              license = licenses.mit;
            };
          });

          cast-mcp-client = pkgs.rustPlatform.buildRustPackage (common // {
            pname = "cast-mcp-client";
            cargoBuildFlags = [ "-p" "cast-mcp-client" ];
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
              pkgs.rust-analyzer
              pkgs.cargo-expand
              pkgs.cargo-watch
              pkgs.cargo-edit
            ];

            shellHook = ''
              echo "Rust development environment ready!"
              echo "Rust version: $(rustc --version)"
            '';
          };
      });
}
