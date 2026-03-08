{
  description = "authentik platform";

  nixConfig = {
    extra-substituters = [ "https://pr-525--authentik-pkg.netlify.app/nix" ];
    extra-trusted-public-keys = [
      "authentik-pkg:ZZHUD/9SkS8T1BVVoksE/+QjIo0s3F8/AM/h0J3ckaw="
    ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      cargoVersion = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
      version = "0.40.5";
      vendorHash = "sha256-RosEV9rlG46zvHQVmC64haOcZZo5b/LWu0EgN0IEtdA=";
      mkOverlay = final: prev:
        let
          system = prev.stdenv.hostPlatform.system;
        in
        {
          authentik-cli = self.packages.${system}.ak-cli;
          authentik-sysd = self.packages.${system}.ak-sysd;
          authentik-agent = self.packages.${system}.ak-agent;
          authentik-browser-support = self.packages.${system}.ak-browser-support;
        } // prev.lib.optionalAttrs prev.stdenv.isLinux {
          authentik-pam = self.packages.${system}.libpam-authentik;
          authentik-nss = self.packages.${system}.libnss-authentik;
        };
    in
    assert version == cargoVersion;
    flake-utils.lib.eachSystem systems
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
          lib = pkgs.lib;
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" ];
          };
          buildHash = if self ? rev then self.rev else "dirty";
          commonGoArgs = {
            buildGoModule = pkgs.buildGo126Module;
            inherit version vendorHash;
            goBuildInputs = with pkgs; [
              pkg-config
            ] ++ lib.optionals pkgs.stdenv.isDarwin [
              libiconv
            ];
            goLdflags = [
              "-s"
              "-w"
              "-X goauthentik.io/platform/pkg/meta.Version=${version}"
              "-X goauthentik.io/platform/pkg/meta.BuildHash=${buildHash}"
            ];
          };
          mkGoPackage = path: pkgs.callPackage path commonGoArgs;
          basePackages = rec {
            ak-cli = mkGoPackage ./nix/packages/ak-cli.nix;
            ak-sysd = mkGoPackage ./nix/packages/ak-sysd.nix;
            ak-agent = mkGoPackage ./nix/packages/ak-agent.nix;
            ak-browser-support = mkGoPackage ./nix/packages/ak-browser-support.nix;
            default = ak-agent;
          };
          linuxPackages = lib.optionalAttrs pkgs.stdenv.isLinux {
            libpam-authentik = pkgs.callPackage ./nix/packages/libpam-authentik.nix {
              inherit version;
            };
            libnss-authentik = pkgs.callPackage ./nix/packages/libnss-authentik.nix {
              inherit version;
            };
          };
          packages = basePackages // linuxPackages;
        in
        {
          inherit packages;

          apps.default = {
            type = "app";
            program = "${packages.default}/bin/ak-agent";
          };

          checks = packages;

          devShells.default = pkgs.mkShell {
            buildInputs = with pkgs; [
              go_1_26
              gopls
              gotools
              go-tools
              rustToolchain
              rust-analyzer
              protobuf
              protoc-gen-go
              protoc-gen-go-grpc
              pkg-config
              openssl
              nodejs
              gnumake
            ] ++ lib.optionals pkgs.stdenv.isLinux [
              pam
            ] ++ lib.optionals pkgs.stdenv.isDarwin [
              libiconv
            ];

            shellHook = ''
              echo "authentik platform development shell"
              echo "Go: $(go version)"
              echo "Rust: $(rustc --version)"
            '';
          };

          formatter = pkgs.nixpkgs-fmt;
        }) // {
      nixosModules.default = import ./nix/modules/authentik.nix;
      nixosModules.authentik = self.nixosModules.default;

      darwinModules.default = import ./nix/modules/darwin.nix;
      darwinModules.authentik = self.darwinModules.default;

      overlays.default = mkOverlay;
    };
}
