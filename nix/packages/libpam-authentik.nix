{ lib
, stdenv
, rustPlatform
, pkg-config
, pam
, openssl
, version
}:

rustPlatform.buildRustPackage {
  pname = "libpam-authentik";
  inherit version;

  src = ../..;

  cargoLock = {
    lockFile = ../../Cargo.lock;
  };

  # Build only the PAM module
  buildAndTestSubdir = "pam";

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    pam
    openssl
  ];

  postInstall = ''
    # Install the PAM module to the correct location
    mkdir -p $out/lib/security
    cp target/${stdenv.hostPlatform.rust.rustcTarget}/release/libauthentik_pam.so $out/lib/security/pam_authentik.so

    # Install PAM configuration for pam-auth-update
    mkdir -p $out/share/pam-configs
    cat > $out/share/pam-configs/authentik << 'EOF'
    Name: authentik Authentication
    Default: yes
    Priority: 512
    Auth-Type: Primary
    Auth:
            [success=end default=ignore]    pam_authentik.so
    Auth-Initial:
            [success=end default=ignore]    pam_authentik.so
    Session-Type: Additional
    Session:
            required                        pam_authentik.so
    EOF
  '';

  meta = with lib; {
    description = "PAM module for authentik authentication";
    homepage = "https://goauthentik.io";
    license = licenses.mit;
    maintainers = [ ];
    platforms = platforms.linux;
  };
}
