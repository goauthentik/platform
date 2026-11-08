# Renewing Apple signing certificates

The macOS builds are signed with two Developer ID certificates for team `232G855Y8N`. Both are valid for 5 years.

| Certificate                                                     | Signs                                        | GitHub secret               |
| --------------------------------------------------------------- | -------------------------------------------- | --------------------------- |
| `Developer ID Application: Authentik Security Inc. (232G855Y8N)` | `authentik Agent.app`, its binaries, PSSO    | `APPLE_SIGN_CERT`           |
| `Developer ID Installer: Authentik Security Inc. (232G855Y8N)`   | `authentik Agent Installer.pkg`              | `APPLE_SIGN_CERT_INSTALLER` |

Both p12 files are imported with the same password, `APPLE_SIGN_CERT_PASSWORD`.

Notarization and provisioning profile downloads use an App Store Connect API key (`APPLE_AC_ISSUER`, `APPLE_AC_KID`, `APPLE_AC_KEY`). API keys don't expire, so they only need replacing if the key is revoked.

Releases that were already shipped keep working after a certificate expires, because every signature carries a secure timestamp. Renewing only matters for new builds.

## Check expiry dates

```shell
security find-certificate -c "Developer ID Application: Authentik Security Inc." -p | openssl x509 -noout -enddate
security find-certificate -c "Developer ID Installer: Authentik Security Inc." -p | openssl x509 -noout -enddate
```

The dates are also listed under [Certificates](https://developer.apple.com/account/resources/certificates/list) in the developer portal.

## 1. Create the new certificates

Only the Account Holder of the Apple developer team can create Developer ID certificates.

1. In Keychain Access, open **Keychain Access → Certificate Assistant → Request a Certificate From a Certificate Authority…**. Enter the email address, choose **Saved to disk**, and save the `.certSigningRequest`.
2. In the developer portal, open [Certificates](https://developer.apple.com/account/resources/certificates/list) and click **+**.
3. Choose **Developer ID Application** and select the **G2 Sub-CA** profile. Upload the CSR and download the `.cer`.
4. Repeat steps 1 to 3 for **Developer ID Installer**. The portal rejects a CSR that has already been used, so create a new one.
5. Double-click both `.cer` files so they are added to the login keychain alongside the private key.

Don't revoke the old certificates. A revoked Developer ID certificate invalidates every release it signed, while an expired one does not.

## 2. Export the certificates as p12

For each certificate:

1. In Keychain Access, under **login → My Certificates**, expand the certificate so that its private key is shown.
2. Select both the certificate and the key, then choose **File → Export Items…** and save as `.p12`.
3. Use the current `APPLE_SIGN_CERT_PASSWORD` as the export password. If you set a new password, update that secret too: both p12 files must use the same one.

Keep the p12 files somewhere safe, such as the team password manager, and delete them from disk afterwards.

## 3. Update the GitHub secrets

```shell
base64 -i application.p12 | gh secret set APPLE_SIGN_CERT --repo goauthentik/platform
base64 -i installer.p12 | gh secret set APPLE_SIGN_CERT_INSTALLER --repo goauthentik/platform
# only if the password changed
gh secret set APPLE_SIGN_CERT_PASSWORD --repo goauthentik/platform
```

## 4. Regenerate the provisioning profiles

The Developer ID provisioning profiles are bound to the Application certificate, so they have to be regenerated with the new one. CI downloads them with `apple-actions/download-provisioning-profiles` (`MAC_APP_DIRECT`) for:

- `io.goauthentik.platform.agent`, which `vpkg/macos/Makefile` copies into the app as `embedded.provisionprofile`
- `io.goauthentik.platform.psso`, which Xcode looks up by the name `GHA authentik Platform PSSO`

For each profile, open [Profiles](https://developer.apple.com/account/resources/profiles/list), edit it, select the new **Developer ID Application** certificate, and save. Keep the PSSO profile's name unchanged, or update `PROVISIONING_PROFILE_SPECIFIER` in `ee/psso/authentikPlatform.xcodeproj/project.pbxproj` to match.

## 5. Update the hardcoded certificate hash

`vpkg/macos/Makefile` selects the Application certificate by its SHA-1 hash (`MACOS_SIGN_CERT`). The hash changes with every new certificate:

```shell
security find-identity -v -p codesigning | grep "Developer ID Application"
```

Replace `MACOS_SIGN_CERT` with the new hash.

The Installer certificate is selected by name (`MACOS_INSTALLER_CERT` in `.github/workflows/_package-darwin.yml`), so nothing changes for it. If the old and new certificates are both in the same keychain, `productbuild` may pick the wrong one. Remove the expired certificate from local keychains.

## 6. Local development

Developers who build signed packages locally (`make vpkg/macos/local`) need to:

1. Import the new Application p12 into their login keychain.
2. Download the regenerated `io.goauthentik.platform.agent` profile from the portal and double-click it. It is installed to `~/Library/Developer/Xcode/UserData/Provisioning Profiles/`. Delete the old profile from that folder, because the Makefile's `cp` fails if more than one profile matches the bundle ID.

## 7. Verify

Run the `build` workflow on a branch, then download the `authentik_package-macos` artifact:

```shell
pkgutil --check-signature "authentik Agent Installer.pkg"     # new Installer cert, notarized
spctl -a -vvv -t install "authentik Agent Installer.pkg"      # source=Notarized Developer ID
xcrun stapler validate "authentik Agent Installer.pkg"
```

If notarization fails, run `xcrun notarytool log <submission-id>` with the App Store Connect key to see Apple's reason.
