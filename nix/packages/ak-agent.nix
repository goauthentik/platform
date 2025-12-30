{ lib
, buildGoModule
, version
, vendorHash
, goLdflags
, goBuildInputs
, stdenv
}:

buildGoModule {
  pname = "ak-agent";
  inherit version vendorHash;

  src = ../..;

  subPackages = [
    "cmd/agent_local"
    "cmd/agent_system"
    "cmd/cli"
    "cmd/browser_support"
  ];

  nativeBuildInputs = goBuildInputs;

  ldflags = goLdflags;

  postInstall = let
    # Reference source root - the space in "authentik Agent.app" is handled in the string
    srcRoot = ../..;
  in ''
    # Rename binaries
    mv $out/bin/agent_local $out/bin/ak-agent
    mv $out/bin/agent_system $out/bin/ak-sysd
    mv $out/bin/cli $out/bin/ak
    mv $out/bin/browser_support $out/bin/ak-browser-support
  '' + lib.optionalString stdenv.isLinux ''
    # Install user systemd service file from source
    mkdir -p $out/lib/systemd/user
    sed "s|/usr/bin/ak-agent|$out/bin/ak-agent|g" \
      "${srcRoot}/cmd/agent_local/package/linux/etc/systemd/user/ak-agent.service" \
      > $out/lib/systemd/user/ak-agent.service

    # Install polkit policy
    mkdir -p $out/share/polkit-1/actions
    cp "${srcRoot}/cmd/agent_local/package/linux/usr/share/polkit-1/actions/io.goauthentik.platform.policy" \
      $out/share/polkit-1/actions/
  '' + lib.optionalString stdenv.isDarwin ''
    # Create macOS .app bundle
    APP_DIR="$out/Applications/authentik Agent.app/Contents"
    mkdir -p "$APP_DIR/MacOS"
    mkdir -p "$APP_DIR/Resources"
    mkdir -p "$out/Library/LaunchDaemons"

    # Copy all binaries into the app bundle
    cp $out/bin/ak-agent "$APP_DIR/MacOS/ak-agent"
    cp $out/bin/ak-sysd "$APP_DIR/MacOS/ak-sysd"
    cp $out/bin/ak "$APP_DIR/MacOS/ak"
    cp $out/bin/ak-browser-support "$APP_DIR/MacOS/ak-browser-support"

    # Copy resources from source
    cp "${srcRoot}/cmd/agent_local/package/macos/authentik Agent.app/Contents/Resources/icon.icns" "$APP_DIR/Resources/"
    cp "${srcRoot}/cmd/agent_local/package/macos/authentik Agent.app/Contents/Resources/browser-host-chrome.json" "$APP_DIR/Resources/"
    cp "${srcRoot}/cmd/agent_local/package/macos/authentik Agent.app/Contents/Resources/browser-host-firefox.json" "$APP_DIR/Resources/"

    # Copy Info.plist from source and substitute version
    sed "s|0.35.2|${version}|g" \
      "${srcRoot}/cmd/agent_local/package/macos/authentik Agent.app/Contents/Info.plist" \
      > "$APP_DIR/Info.plist"

    # Copy launchd plist from source (daemon.plist)
    cp "${srcRoot}/cmd/agent_local/package/macos/scripts/daemon.plist" \
      "$out/Library/LaunchDaemons/io.goauthentik.platform.sysd.plist"
  '';

  meta = with lib; {
    description = "authentik Local Agent for user authentication";
    homepage = "https://goauthentik.io";
    license = licenses.mit;
    maintainers = [ ];
    platforms = platforms.linux ++ platforms.darwin;
    mainProgram = "ak-agent";
  };
}
