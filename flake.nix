{
  description = "fanschnick";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }: flake-utils.lib.eachDefaultSystem (system: 
    let
      pkgs = import nixpkgs {
        system = system;
        overlays = [
          rust-overlay.overlays.default
        ];
      };
      rustPlatform = pkgs.makeRustPlatform rec {
        cargo = pkgs.rust-bin.stable.latest.default;
        rustc = cargo;
      };
      cargoToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml));
    in
    rec {
      packages = rec {
        default = fanschnick-server;
        fanschnick-server = rustPlatform.buildRustPackage {
            name = "fanschnick-server";
            version = cargoToml.package.version;
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            buildInputs = [ pkgs.postgresql ];
        };
      };
      apps = rec {
        default = fanschnick-server;
        fanschnick-server = flake-utils.lib.mkApp {
          drv = packages.fanschnick-server; 
        };
      };
    }
  );
}