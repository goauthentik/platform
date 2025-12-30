{ lib
, stdenv
, rustPlatform
, pkg-config
, openssl
, version
}:

rustPlatform.buildRustPackage {
  pname = "libnss-authentik";
  inherit version;

  src = ../..;

  cargoLock = {
    lockFile = ../../Cargo.lock;
  };

  # Build only the NSS module
  buildAndTestSubdir = "nss";

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    openssl
  ];

  postInstall = ''
    # Install the NSS module (must be named libnss_<name>.so.2)
    mkdir -p $out/lib
    cp target/${stdenv.hostPlatform.rust.rustcTarget}/release/libauthentik_nss.so $out/lib/libnss_authentik.so.2
    ln -s libnss_authentik.so.2 $out/lib/libnss_authentik.so
  '';

  meta = with lib; {
    description = "NSS module for authentik name resolution";
    homepage = "https://goauthentik.io";
    license = licenses.mit;
    maintainers = [ ];
    platforms = platforms.linux;
  };
}
