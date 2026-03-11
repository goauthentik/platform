{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.authentik;
in {
  options.services.authentik = {
    enable = mkEnableOption "authentik platform agent";

    package = mkOption {
      type = types.package;
      default = pkgs.authentik-sysd or (throw "authentik-sysd package not found. Add the authentik overlay to your nixpkgs.");
      defaultText = literalExpression "pkgs.authentik-sysd";
      description = "The authentik sysd package to use.";
    };

    cliPackage = mkOption {
      type = types.package;
      default = pkgs.authentik-cli or (throw "authentik-cli package not found. Add the authentik overlay to your nixpkgs.");
      defaultText = literalExpression "pkgs.authentik-cli";
      description = "The authentik CLI package to use.";
    };

    agentPackage = mkOption {
      type = types.package;
      default = pkgs.authentik-agent or (throw "authentik-agent package not found. Add the authentik overlay to your nixpkgs.");
      defaultText = literalExpression "pkgs.authentik-agent";
      description = "The authentik agent package to use.";
    };

    domain = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "The authentik server domain to connect to.";
    };

    configFile = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = "Path to the authentik configuration file.";
    };

    enablePAM = mkOption {
      type = types.bool;
      default = false;
      description = "Enable PAM authentication via authentik.";
    };

    pamPackage = mkOption {
      type = types.package;
      default = pkgs.authentik-pam or (throw "authentik-pam package not found. Add the authentik overlay to your nixpkgs.");
      defaultText = literalExpression "pkgs.authentik-pam";
      description = "The authentik PAM module package to use.";
    };

    pamServices = mkOption {
      type = types.listOf types.str;
      default = [ "login" "sshd" "sudo" ];
      description = "List of PAM services to enable authentik authentication for.";
    };

    enableNSS = mkOption {
      type = types.bool;
      default = false;
      description = "Enable NSS name resolution via authentik.";
    };

    nssPackage = mkOption {
      type = types.package;
      default = pkgs.authentik-nss or (throw "authentik-nss package not found. Add the authentik overlay to your nixpkgs.");
      defaultText = literalExpression "pkgs.authentik-nss";
      description = "The authentik NSS module package to use.";
    };

    extraArgs = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "Extra command-line arguments to pass to ak-sysd.";
    };
  };

  config = mkIf cfg.enable {
    # Install CLI and agent packages
    environment.systemPackages = [
      cfg.cliPackage
      cfg.agentPackage
    ];

    # Create configuration directory
    environment.etc."authentik/.keep".text = "";

    # Systemd service for ak-sysd
    systemd.services.ak-sysd = {
      description = "authentik System Agent";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/ak-sysd agent ${concatStringsSep " " cfg.extraArgs}";
        Restart = "always";
        RestartSec = 5;
        RuntimeDirectory = "authentik";
        RuntimeDirectoryMode = "0777";
        StateDirectory = "authentik";
        ConfigurationDirectory = "authentik";
      };

      environment = mkIf (cfg.domain != null) {
        AUTHENTIK_DOMAIN = cfg.domain;
      };
    };

    # User systemd service for ak-agent (optional, user must enable)
    systemd.user.services.ak-agent = {
      description = "authentik Local Agent";
      after = [ "graphical-session.target" ];
      wantedBy = [ "default.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.agentPackage}/bin/ak-agent";
        Restart = "always";
        RestartSec = 5;
      };
    };

    # PAM configuration
    security.pam.services = mkIf cfg.enablePAM (
      genAttrs cfg.pamServices (service: {
        text = mkAfter ''
          # authentik PAM authentication
          auth     sufficient  ${cfg.pamPackage}/lib/security/pam_authentik.so
          session  required    ${cfg.pamPackage}/lib/security/pam_authentik.so
        '';
      })
    );

    # NSS configuration
    system.nssModules = mkIf cfg.enableNSS [ cfg.nssPackage ];
    system.nssDatabases.passwd = mkIf cfg.enableNSS [ "authentik" ];
    system.nssDatabases.group = mkIf cfg.enableNSS [ "authentik" ];

    # Add NSS library to the dynamic linker path
    environment.etc."ld.so.conf.d/authentik-nss.conf" = mkIf cfg.enableNSS {
      text = "${cfg.nssPackage}/lib";
    };

    # Ensure ldconfig is run after configuration changes
    system.activationScripts.authentik-ldconfig = mkIf cfg.enableNSS ''
      ${pkgs.glibc.bin}/bin/ldconfig
    '';
  };
}
