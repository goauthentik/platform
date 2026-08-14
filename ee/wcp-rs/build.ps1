Set-PSDebug -Trace 1
$ErrorActionPreference = "Stop"

if ($env:CI -ne "true") {
    . "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\Launch-VsDevShell.ps1" -arch amd64
}

$pwd = $args[0]
$top = $args[1]
$target = "wcp"

New-Item -ea 0 -Force -ItemType Directory "$top/bin/$target"

$cefPin = (Select-String -Path "$pwd/Cargo.lock" -Pattern '^name = "cef-dll-sys"' -Context 0, 1).Context.PostContext[0] `
    -replace '.*"([0-9.]+)\+.*', '$1'
$cefPath = "$top/cache/cef"

cargo install export-cef-dir --version $cefPin --locked
& export-cef-dir --force $cefPath

$env:CEF_PATH = $cefPath
cargo build --release --target-dir "$top/cache/wcp-rs" --manifest-path "$pwd/Cargo.toml"

$release = "$top/cache/wcp-rs/release"
Copy-Item "$release/ak_cred_provider.dll" "$top/bin/$target/"
Copy-Item "$release/ak_cred_provider.pdb" "$top/bin/$target/" -ErrorAction SilentlyContinue
Copy-Item "$release/ak_cred_provider.dll.lib" "$top/bin/$target/ak_cred_provider.lib"
Copy-Item "$release/ak_cef.exe" "$top/bin/$target/"
Copy-Item "$release/ak_cef.pdb" "$top/bin/$target/" -ErrorAction SilentlyContinue
Copy-Item "$cefPath/*.dll" "$top/bin/$target/"
Copy-Item "$cefPath/*.pak" "$top/bin/$target/"
Copy-Item "$cefPath/icudtl.dat" "$top/bin/$target/"
Copy-Item "$cefPath/v8_context_snapshot.bin" "$top/bin/$target/"
Copy-Item "$cefPath/vk_swiftshader_icd.json" "$top/bin/$target/"
Copy-Item "$cefPath/locales" "$top/bin/$target/" -Recurse -Force
