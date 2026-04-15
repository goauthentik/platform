{ lib
, buildGoModule
, version
, vendorHash
, goLdflags
, goBuildInputs
, stdenv
}:

buildGoModule {
  pname = "ak-browser-support";
  inherit version vendorHash;

  src = ../..;

  subPackages = [ "cmd/browser_support" ];

  nativeBuildInputs = goBuildInputs;

  ldflags = goLdflags;

  postInstall = let
    srcRoot = ../..;
  in ''
    mv $out/bin/browser_support $out/bin/ak-browser-support

    # Include browser native messaging manifests in share for modules to install
    mkdir -p $out/share/authentik
    sed "s|/usr/bin/ak-browser-support|$out/bin/ak-browser-support|g" \
      "${srcRoot}/cmd/agent_system/package/linux/browser-host-chrome.json" \
      > $out/share/authentik/browser-host-chrome.json
    sed "s|/usr/bin/ak-browser-support|$out/bin/ak-browser-support|g" \
      "${srcRoot}/cmd/agent_system/package/linux/browser-host-firefox.json" \
      > $out/share/authentik/browser-host-firefox.json
  '';

  meta = with lib; {
    description = "authentik Browser Support for native messaging";
    homepage = "https://goauthentik.io";
    license = licenses.mit;
    maintainers = [ ];
    platforms = platforms.linux ++ platforms.darwin;
    mainProgram = "ak-browser-support";
  };
}
