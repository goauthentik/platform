Set-PSDebug -Trace 1
$ErrorActionPreference = "Stop"

$rust = Resolve-Path "~\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin"
$make = "C:\Program Files (x86)\GnuWin32\bin"
$perl = "C:\Strawberry\perl\bin\"

$env:Path = "$perl;$rust;$make;$env:Path"
