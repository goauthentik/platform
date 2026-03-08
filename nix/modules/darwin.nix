{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.authentik;
  appPath = "/Applications/authentik Agent.app";
  packageAppPath = "${cfg.package}/Applications/authentik Agent.app";
  browserSupportPath = "${packageAppPath}/Contents/MacOS/ak-browser-support";
  defaultConfigPath = "/opt/authentik/config/config.json";
  targetConfigPath = if cfg.configFile != null then toString cfg.configFile else defaultConfigPath;
  defaultSysdConfig = ../../cmd/agent_local/package/macos/scripts/sysd-config.json;
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

    # Expose the .app bundle at the upstream path and install browser native messaging hosts.
    system.activationScripts.authentik-agent.text = ''
      echo "Installing authentik Agent.app..."
      rm -rf "${appPath}"
      ln -sfn "${packageAppPath}" "${appPath}"
      mkdir -p /Library/Logs/io.goauthentik
      chmod 755 /Library/Logs/io.goauthentik

      echo "Preparing authentik runtime directories..."
      mkdir -p /opt/authentik/config
      mkdir -p /opt/authentik/domains
      mkdir -p /opt/authentik/runtime
      chmod 700 /opt/authentik
      chmod 700 /opt/authentik/config
      chmod 700 /opt/authentik/domains
      chmod 700 /opt/authentik/runtime

      if [ ! -e "${targetConfigPath}" ]; then
        echo "Seeding default authentik sysd config..."
        mkdir -p "$(dirname "${targetConfigPath}")"
        install -m 600 "${defaultSysdConfig}" "${targetConfigPath}"
      fi

      echo "Installing browser native messaging hosts..."
      # Chrome/Chromium (system-wide)
      mkdir -p "/Library/Google/Chrome/NativeMessagingHosts"
      sed "s|/Applications/authentik Agent.app/Contents/MacOS/ak-browser-support|${browserSupportPath}|g" \
         "${appPath}/Contents/Resources/browser-host-chrome.json" \
         > "/Library/Google/Chrome/NativeMessagingHosts/io.goauthentik.platform.json"

      mkdir -p "/Library/Application Support/Chromium/NativeMessagingHosts"
      sed "s|/Applications/authentik Agent.app/Contents/MacOS/ak-browser-support|${browserSupportPath}|g" \
         "${appPath}/Contents/Resources/browser-host-chrome.json" \
         > "/Library/Application Support/Chromium/NativeMessagingHosts/io.goauthentik.platform.json"

      # Edge (system-wide)
      mkdir -p "/Library/Microsoft/Edge/NativeMessagingHosts"
      sed "s|/Applications/authentik Agent.app/Contents/MacOS/ak-browser-support|${browserSupportPath}|g" \
         "${appPath}/Contents/Resources/browser-host-chrome.json" \
         > "/Library/Microsoft/Edge/NativeMessagingHosts/io.goauthentik.platform.json"

      # Firefox (system-wide)
      mkdir -p "/Library/Application Support/Mozilla/NativeMessagingHosts"
      sed "s|/Applications/authentik Agent.app/Contents/MacOS/ak-browser-support|${browserSupportPath}|g" \
         "${appPath}/Contents/Resources/browser-host-firefox.json" \
         > "/Library/Application Support/Mozilla/NativeMessagingHosts/io.goauthentik.platform.json"
    '';

    # Launchd daemon for ak-sysd (runs as root)
    launchd.daemons.io-goauthentik-platform-sysd = {
      serviceConfig = {
        Label = "io.goauthentik.platform.sysd";
        ProgramArguments = [
          "${packageAppPath}/Contents/MacOS/ak-sysd"
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
