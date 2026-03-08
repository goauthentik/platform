{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.authentik;
in
{
  options.services.authentik = {
    enable = mkEnableOption "authentik platform agent";

    package = mkOption {
      type = types.package;
      default = pkgs.authentik-agent or (throw "authentik-agent package not found. Add the authentik overlay to your nixpkgs.");
      defaultText = literalExpression "pkgs.authentik-agent";
      description = "The authentik agent package to use (includes all binaries).";
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

    extraArgs = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "Extra command-line arguments to pass to ak-sysd.";
    };
  };

  config = mkIf cfg.enable {
    # Install the package (makes binaries available in PATH)
    environment.systemPackages = [ cfg.package ];

    # Copy .app bundle to /Applications and install browser native messaging hosts
    system.activationScripts.authentik-agent.text = ''
      echo "Installing authentik Agent.app..."
      rm -rf "/Applications/authentik Agent.app"
      cp -R "${cfg.package}/Applications/authentik Agent.app" "/Applications/"
      chmod -R 755 "/Applications/authentik Agent.app"
      mkdir -p /Library/Logs/io.goauthentik

      echo "Installing browser native messaging hosts..."
      # Chrome/Chromium (system-wide)
      mkdir -p "/Library/Google/Chrome/NativeMessagingHosts"
      cp "/Applications/authentik Agent.app/Contents/Resources/browser-host-chrome.json" \
         "/Library/Google/Chrome/NativeMessagingHosts/io.goauthentik.platform.json"

      # Edge (system-wide)
      mkdir -p "/Library/Microsoft/Edge/NativeMessagingHosts"
      cp "/Applications/authentik Agent.app/Contents/Resources/browser-host-chrome.json" \
         "/Library/Microsoft/Edge/NativeMessagingHosts/io.goauthentik.platform.json"

      # Firefox (system-wide)
      mkdir -p "/Library/Application Support/Mozilla/NativeMessagingHosts"
      cp "/Applications/authentik Agent.app/Contents/Resources/browser-host-firefox.json" \
         "/Library/Application Support/Mozilla/NativeMessagingHosts/io.goauthentik.platform.json"
    '';

    # Launchd daemon for ak-sysd (runs as root)
    launchd.daemons.io-goauthentik-platform-sysd = {
      serviceConfig = {
        Label = "io.goauthentik.platform.sysd";
        ProgramArguments = [
          "${cfg.package}/Applications/authentik Agent.app/Contents/MacOS/ak-sysd"
          "agent"
        ] ++ lib.optionals (cfg.configFile != null) [
          "--config-file"
          (toString cfg.configFile)
        ] ++ cfg.extraArgs;
        EnvironmentVariables = mkIf (cfg.domain != null) {
          AUTHENTIK_DOMAIN = cfg.domain;
        };
        UserName = "root";
        RunAtLoad = true;
        KeepAlive = true;
        StandardOutPath = "/Library/Logs/io.goauthentik/sysd.log";
        StandardErrorPath = "/Library/Logs/io.goauthentik/sysd.log";
      };
    };
  };
}
