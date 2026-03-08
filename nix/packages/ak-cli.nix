{ lib
, buildGoModule
, version
, vendorHash
, goLdflags
, goBuildInputs
}:

buildGoModule {
  pname = "ak-cli";
  inherit version vendorHash;

  src = ../..;

  subPackages = [ "cmd/cli" ];

  nativeBuildInputs = goBuildInputs;

  ldflags = goLdflags;

  postInstall = ''
    mv $out/bin/cli $out/bin/ak

    # Match the packaged CLI entrypoint aliases.
    ln -s $out/bin/ak $out/bin/ak-vault
  '';

  meta = with lib; {
    description = "authentik Platform CLI";
    homepage = "https://goauthentik.io";
    license = licenses.mit;
    maintainers = [ ];
    platforms = platforms.linux ++ platforms.darwin;
    mainProgram = "ak";
  };
}
