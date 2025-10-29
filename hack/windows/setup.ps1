$rust = Resolve-Path "~\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin"
$make = "C:\Program Files (x86)\GnuWin32\bin"
$go = "C:\Program Files\Go\bin"
$perl = "C:\Strawberry\perl\bin\"

$env:Path = "$perl;$rust;$make;$go;$env:Path"
. 'C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\Launch-VsDevShell.ps1'
