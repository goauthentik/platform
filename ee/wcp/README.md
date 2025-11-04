# Credential Provider

## Build tools and test environment

Visual Studio 17 2022

Microsoft Visual Studio Installer Projects 2022 2.0.1 (to package into Setup)

Strawberry Perl (to build OpenSSL for jwt-cpp)
    (`winget install StrawberryPerl.StrawberryPerl`)

Windows 10 x64

### Build steps

#### Development

```
mkdir build
cd build
cmake -G "Visual Studio 17" ..
python ../.utils/addsetup.py
cmake --build . --config Release
```

Alternatively, instead of the last step, open the Visual Studio solution file `cef.sln` inside the *build* directory and build the solution (F7) inside the Visual Studio.

The credential provider files are generated inside the `build/ak_cred_provider/Release` subdirectory.

#### Installer package

**Update:** With the python patch script `addsetup.py` in the `.utils` directory, manually performing this step can be avoided. The following information is provided as an alternate method but should not be necessary anymore.

Inside the *build* directory, open the Visual Studio solution file `cef.sln`.

Right click the `cef` solution in the *Solution Explorer* sub-window and click `Add->Existing Project...`.

Select the Setup project file inside the **build** directory in path `build/Setup/Setup.vdproj`.

`Do not` select the Setup file inside the project source under the *Setup* subdirectory.

Build solution (F7) (again), or right-click the `Setup` project in the *Solution Explorer* sub-window and click `Build`.

The setup package files are available in `build/Release` subdirectory.

#### Testing

During development, the registry setup files inside the `ak_cred_provider/Setup` path could be used to register the credential provide inside the `build` directory for testing.

The setup file/ installer registers the credential provider similarly, so **take note** that either the credential provider inside the build directory or the credential provider installed via the Setup file could be displayed at the logon prompt UI at a time and not both simultaneously.

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
