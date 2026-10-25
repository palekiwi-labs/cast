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
          cast-tui = pkgs.stdenv.mkDerivation {
            pname = "cast-tui";
            version = castTuiPackage.version;
            src = pkgs.lib.cleanSourceWith {
              src = ./cast-tui;
              filter = path: type:
                let base = baseNameOf path; in
                !(builtins.elem base [ "node_modules" "dist" "build-test" ]) &&
                (pkgs.lib.cleanSourceFilter path type);
            };
            # importNpmLock reads package-lock.json at eval time, so there is
            # no npmDepsHash to maintain (pattern from pi's nix/package.nix).
            npmDeps = pkgs.importNpmLock { npmRoot = ./cast-tui; };
            npmRebuildFlags = [ "--ignore-scripts" ];

            nativeBuildInputs = [
              pkgs.nodejs
              pkgs.importNpmLock.npmConfigHook
              pkgs.makeWrapper
            ] ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
              pkgs.autoPatchelfHook
            ];
            # pi-tui ships prebuilt native addons; the Linux one links libxcb.
            buildInputs = pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
              pkgs.stdenv.cc.cc.lib
              pkgs.libxcb
            ];
            dontStrip = true;

            buildPhase = ''
              runHook preBuild
              npm run build
              runHook postBuild
            '';

            doCheck = true;
            checkPhase = ''
              runHook preCheck
              npm test
              runHook postCheck
            '';

            installPhase = ''
              runHook preInstall
              npm prune --omit=dev --ignore-scripts
              find node_modules -type d -empty -delete
              mkdir -p "$out/lib/cast-tui" "$out/bin"
              cp -R package.json bin dist node_modules "$out/lib/cast-tui"
              makeWrapper ${pkgs.nodejs}/bin/node "$out/bin/cast-tui" \
                --add-flags "$out/lib/cast-tui/bin/cast-tui.js"
              runHook postInstall
            '';

            doInstallCheck = true;
            installCheckPhase = ''
              runHook preInstallCheck
              # Load the host-platform pi-tui addons so a missing native
              # library fails the build instead of at runtime.
              ${pkgs.nodejs}/bin/node -e "
                const fs = require('node:fs'), path = require('node:path');
                const p = process.platform, dir = '$out/lib/cast-tui/node_modules/@earendil-works/pi-tui/native/' + p + '/prebuilds/' + p + '-' + process.arch;
                if (fs.existsSync(dir)) for (const f of fs.readdirSync(dir)) if (f.endsWith('.node')) require(path.join(dir, f));
              "
              runHook postInstallCheck
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
