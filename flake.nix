{
  description = "fanschnick";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }: 
  flake-utils.lib.eachDefaultSystem (system: 
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
            nativeBuildInputs = [ pkgs.postgresql ];
            postInstall = ''
              cp -r assets/ $out/assets
            '';
        };
      };
      apps = rec {
        default = fanschnick-server;
        fanschnick-server = flake-utils.lib.mkApp {
          drv = packages.fanschnick-server; 
        };
      };
    }
  ) // {nixosModules = rec {
    default = fanschnick-server;
    fanschnick-server = { config, lib, pkgs, ... }:
      let cfg = config.services.fanschnick-server; in {
        options = with lib.types; {
        services.fanschnick-server = {
          enable = lib.mkEnableOption "Enable Fanschnick";
          host = lib.mkOption { type = str; description = "Virtual Host to serve under"; };
          port = lib.mkOption { type = port; default = 8080; description = "Local port to bind to"; };
        };
        };
        config.services.postgresql = lib.mkIf cfg.enable {
          enable = true;
          package = pkgs.postgresql_18;
          ensureDatabases = [ "fanschnick" ];
          ensureUsers = [{
            name = "fanschnick";
            ensureDBOwnership = true;
            ensureClauses.login = true;
            ensureClauses.createdb = true;
          }];
          authentication = pkgs.lib.mkOverride 10 ''
            #type database  DBuser  auth-method
            local all       all     trust
            host  all       all     127.0.0.1/32 trust
            host  all       all     ::1/128      trust
          '';
        };
        config.services.nginx.virtualHosts = lib.mkIf cfg.enable {
          ${cfg.host} = {
            forceSSL = true;
            enableACME = true;
            locations."/" = {
              proxyPass = "http://127.0.0.1:${builtins.toString cfg.port}"; 
              recommendedProxySettings = true;
            };
            locations."/assets/" = {
              alias = "${self.packages.x86_64-linux.fanschnick-server}/assets/";
              extraConfig = ''
                sendfile   on;
                sendfile_max_chunk 1m;
                tcp_nopush on;
              '';
            };
            locations."/.well-known/".root = "/var/lib/acme/acme-challenge";
          };
        };
        config.users.groups.fanschnick = {};
        config.users.users.fanschnick = lib.mkIf cfg.enable {
          isNormalUser = true;
          group = "fanschnick";
        };
        config.systemd.services.fanschnick-server = lib.mkIf cfg.enable {
          description = "Fanschnick Server";
          wantedBy = [ "multi-user.target" ];
          after = [ "network.target" "postgresql.target" ];
          environment = {
            "DATABASE_URL" = "postgres://localhost/fanschnick";
          };
          serviceConfig = {
            User = "fanschnick";
            Group = "fanschnick";
            ExecStart = "${self.packages.x86_64-linux.fanschnick-server}/bin/fanschnick-server https://${cfg.host} 127.0.0.1:${builtins.toString cfg.port}";
          };
        };
      };
  };};
}