# Credential Provider

## Build tools and test environment

- Visual Studio 17 2022
- Windows 10/11 x64
- Rust (`winget install -e --id Rustlang.Rustup`)
- Make

### Build steps

#### Development

```
make ee/wcp/build
```

The credential provider files are generated inside the `cache/wcp/ak_cred_provider/Release` subdirectory.

#### Testing

Use the authentk Agent from https://pkg.goauthentik.io to install the current stable build of the Agent, and enable the credential provider in the setup wizard. Afterwards, copy the files from `cache/wcp/ak_cred_provider/Release` into the `C:\Program Files\Authentik Security Inc.\wcp\` folder on the target machine, overwriting all files.

## Important points

* Build in Release mode
* Choose appropriate architecture (x86/ x64) as per the Windows platform
* Register the credential provider dll by executing the `Register.reg` file. Make sure to set the correct path to the credential provider dll inside the registry file. Use absolute path with double slashes `\\`.
* Sign the dll, if required. A test certificate may be used temporarily though this is not a pre-requisite.
* If using installer/ setup, make sure to install for "Everyone" not "Just me". (not applicable, installer pre-configured).
* If you change the credential provider GUID, make sure to change it in *guid.h* and also update the Setup project file `Setup/Setup.vdproj`.
* `Note:` If you make any changes to the Setup project during development, make sure the Setup file changes in the build directory `build/Setup/Setup.vdproj` are added to the Setup file in the source directory `Setup/Setup.vdproj` and committed to the repository.

## Removing the credential provider

* Unregister by removing the registry entries, or execute `Unregister.reg`
* The credential provider added via the Setup file could be uninstalled by relaunching the Setup file or from the `Programs and Features` utility inside the Windows `Control Panel`.
* If locked out of PC, use a bootable disk to repair/ troubleshoot using command prompt. Unregister the dll in registry or otherwise `cd` to the credential provider directory and rename or remove the dll. This credential provider would then not be loaded at logon.

# To Do:

Recheck character encoding (char vs wide char, Unicode vs ASCII etc)

Specify architecture (32/64)-bit in build commands to build for each architecture

Specify architecture (32/64)-bit in build commands for OpenSSL through Perl configure

Clean up and overwrite JWT objects etc for (memory) security
