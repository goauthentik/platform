{ lib
, buildGoModule
, stdenv
, version
, vendorHash
, goLdflags
, goBuildInputs
}:

buildGoModule {
  pname = "ak-sysd";
  inherit version vendorHash;

  src = ../..;

  subPackages = [ "cmd/agent_system" ];

  nativeBuildInputs = goBuildInputs;

  ldflags = goLdflags;

  postInstall = ''
    mv $out/bin/agent_system $out/bin/ak-sysd

    # Install systemd service file (Linux only)
    ${lib.optionalString stdenv.isLinux ''
      mkdir -p $out/lib/systemd/system
      cat > $out/lib/systemd/system/ak-sysd.service << EOF
      [Unit]
      Description=authentik sysd
      After=network.target

      [Service]
      Restart=always
      ExecStart=$out/bin/ak-sysd agent
      RuntimeDirectory=authentik
      RuntimeDirectoryMode=0777

      [Install]
      WantedBy=multi-user.target
      EOF
    ''}

    # Create default config directory structure
    mkdir -p $out/etc/authentik
  '';

  meta = with lib; {
    description = "authentik System Agent daemon";
    homepage = "https://goauthentik.io";
    license = licenses.mit;
    maintainers = [ ];
    platforms = platforms.linux ++ platforms.darwin;
    mainProgram = "ak-sysd";
  };
}
