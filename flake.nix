{
  description = "authentik platform";

  nixConfig = {
    extra-substituters = [ "https://authentik-pkg.netlify.app/nix" ];
    trusted-substituters = [ "https://authentik-pkg.netlify.app/nix" ];
    extra-trusted-public-keys = [ "authentik-pkg:ZZHUD/9SkS8T1BVVoksE/+QjIo0s3F8/AM/h0J3ckaw=" ];
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
    flake-utils.lib.eachSystem [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ] (system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        version = "0.35.4";
        vendorHash = "sha256-qoObvQs5Pk7CYci6PNRzzt797v94ZLTEZpTRRV6QKZM=";

        # Common Go build settings
        goBuildInputs = with pkgs; [
          pkg-config
        ] ++ lib.optionals stdenv.isDarwin [
          libiconv
        ];

        goLdflags = [
          "-s" "-w"
          "-X goauthentik.io/platform/pkg/meta.Version=${version}"
          "-X goauthentik.io/platform/pkg/meta.BuildHash=nix-${self.shortRev or "dirty"}"
        ];

        # Rust toolchain
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
        };

        # Platform checks
        isLinux = pkgs.stdenv.isLinux;
        isDarwin = pkgs.stdenv.isDarwin;

      in {
        packages = {
          ak-cli = pkgs.callPackage ./nix/packages/ak-cli.nix {
            inherit version vendorHash goLdflags goBuildInputs;
          };

          ak-sysd = pkgs.callPackage ./nix/packages/ak-sysd.nix {
            inherit version vendorHash goLdflags goBuildInputs;
          };

          ak-agent = pkgs.callPackage ./nix/packages/ak-agent.nix {
            inherit version vendorHash goLdflags goBuildInputs;
          };

          ak-browser-support = pkgs.callPackage ./nix/packages/ak-browser-support.nix {
            inherit version vendorHash goLdflags goBuildInputs;
          };

          # Rust packages only on Linux
          libpam-authentik = if isLinux then
            pkgs.callPackage ./nix/packages/libpam-authentik.nix {
              inherit version;
            }
          else
            pkgs.runCommand "libpam-authentik-unsupported" {} ''
              echo "libpam-authentik is only available on Linux" >&2
              exit 1
            '';

          libnss-authentik = if isLinux then
            pkgs.callPackage ./nix/packages/libnss-authentik.nix {
              inherit version;
            }
          else
            pkgs.runCommand "libnss-authentik-unsupported" {} ''
              echo "libnss-authentik is only available on Linux" >&2
              exit 1
            '';

          default = self.packages.${system}.ak-agent;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            go
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
          ] ++ lib.optionals isLinux [
            pam
          ] ++ lib.optionals isDarwin [
            libiconv
          ];

          shellHook = ''
            echo "authentik platform development shell"
            echo "Go: $(go version)"
            echo "Rust: $(rustc --version)"
          '';
        };

        # Expose individual package checks
        checks = {
          ak-cli = self.packages.${system}.ak-cli;
        };
      }
    ) // {
      # NixOS module
      nixosModules.default = import ./nix/modules/authentik.nix;
      nixosModules.authentik = self.nixosModules.default;

      # nix-darwin module
      darwinModules.default = import ./nix/modules/darwin.nix;
      darwinModules.authentik = self.darwinModules.default;

      # Overlay for use in other flakes
      overlays.default = final: prev: {
        authentik-cli = self.packages.${prev.system}.ak-cli;
        authentik-sysd = self.packages.${prev.system}.ak-sysd;
        authentik-agent = self.packages.${prev.system}.ak-agent;
        authentik-pam = self.packages.${prev.system}.libpam-authentik;
        authentik-nss = self.packages.${prev.system}.libnss-authentik;
      };
    };
}
