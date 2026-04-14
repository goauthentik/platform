Set-PSDebug -Trace 1
$ErrorActionPreference = "Stop"

if ($env:CI -ne "true") {
    . "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\Launch-VsDevShell.ps1" -arch amd64
}

$pwd = $args[0]
$top = $args[1]
$target = $args[2]

cd $top
New-Item -ea 0 -Force -ItemType Directory "$top/bin/$target"
New-Item -ea 0 -Force -ItemType Directory "$top/cache/$target"
"${top}/hack/windows/setup.ps1"
nmake /P
pwd
cd "$top/cache/$target"
pwd
$env:VERBOSE = "true"
cmake -B"$top/cache/$target" -G "Visual Studio 17" "$top/ee/wcp"
cmake --build . --config Release
