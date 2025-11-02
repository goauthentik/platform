# Authentik Windows Credential Provider (with Chromium Embedded Framework (CEF) integration)

## Build instructions

```
mkdir build
cd build
cmake -G "Visual Studio 17 2022" -DCMAKE_BUILD_TYPE=Release -DCMAKE_CONFIGURATION_TYPES=Release ..
```

Open the project in *Visual Studio 2022* (make sure x64-release or appropriate platform is selected) and build.

## Deployment

Edit `Setup/Register_Partial.reg` to point to the path for the release DLL built in the previous step.
Lock the Windows screen and click `Sign-in options` to use Authentik Windows Credential Provider.

`cefsimple/Release/cefsimple.exe` must be copied to the `libcef_dll_wrapper/Release` directory.

All binaries and resource files in the `Release` and `Resource` directories in the source directory must be copied to the build *libcef_dll_wrapper/Release* directory as well.
