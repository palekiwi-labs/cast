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
        # Package versions are derived from each crate's manifest so
        # Cargo.toml stays the single source of truth; release bumps
        # never need a matching flake.nix edit.
        castCargo = pkgs.lib.importTOML ./crates/cast/Cargo.toml;
        mcpClientCargo = pkgs.lib.importTOML ./crates/cast-mcp-client/Cargo.toml;
        castTuiPackage = pkgs.lib.importJSON ./cast-tui/package.json;
        common = {

          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = path: type:
              let
                relPath = pkgs.lib.removePrefix (toString ./.) (toString path);
              in
              !(pkgs.lib.hasPrefix "/cast-tui" relPath) && (
                (pkgs.lib.hasPrefix "/crates/cast/docs" relPath) ||
                (pkgs.lib.cleanSourceFilter path type)
              );
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
            version = castCargo.package.version;
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
            version = mcpClientCargo.package.version;
            cargoBuildFlags = [ "-p" "cast-mcp-client" ];
            cargoTestFlags = [ "-p" "cast-mcp-client" ];
            nativeCheckInputs = [ pkgs.bash pkgs.jq ];
            meta = with pkgs.lib; {
              description = "Lightweight MCP client for cast";
              homepage = "https://github.com/palekiwi-labs/cast";
              license = licenses.mit;
            };
          });

          # Host TUI for cast. Ships in lockstep with the cast crate; the
          # version-sync check below enforces that.
          cast-tui = pkgs.buildNpmPackage {
            pname = "cast-tui";
            version = castTuiPackage.version;
            src = pkgs.lib.cleanSourceWith {
              src = ./cast-tui;
              filter = path: type:
                let base = baseNameOf path; in
                !(builtins.elem base [ "node_modules" "dist" "build-test" ]) &&
                (pkgs.lib.cleanSourceFilter path type);
            };
            # After changing cast-tui/package-lock.json, set this to
            # pkgs.lib.fakeHash, build, and copy the reported hash.
            npmDepsHash = "sha256-thLJ1Yk1QRple6eHff5ZCONxCxgvZLBViVrVN/AvPqI=";
            nodejs = pkgs.nodejs;
            doCheck = true;
            checkPhase = ''
              runHook preCheck
              npm test
              runHook postCheck
            '';
            meta = with pkgs.lib; {
              description = "Terminal UI for cast";
              homepage = "https://github.com/palekiwi-labs/cast";
              license = licenses.mit;
              mainProgram = "cast-tui";
            };
          };

          default = pkgs.symlinkJoin {
            name = "cast-${castCargo.package.version}";
            paths = [
              self.packages.${system}.cast
              self.packages.${system}.cast-tui
            ];
            meta.mainProgram = "cast";
          };
        };

        checks.version-sync =
          assert pkgs.lib.assertMsg
            (castCargo.package.version == castTuiPackage.version)
            "cast crate (${castCargo.package.version}) and cast-tui (${castTuiPackage.version}) versions differ";
          pkgs.runCommand "cast-version-sync" { } "touch $out";

        devShells.default = pkgs.mkShell
          {
            name = "cast";
            buildInputs = [
              rustToolchain
              pkgs.cargo-expand
              pkgs.cargo-watch
              pkgs.cargo-edit
              pkgs.nodejs
            ];

            shellHook = ''
              echo "Rust development environment ready!" >&2
              echo "Rust version: $(rustc --version)" >&2
            '';
          };
      });
}
