Set-PSDebug -Trace 1
$ErrorActionPreference = "Stop"

if ($env:CI -ne "true") {
    . "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\Launch-VsDevShell.ps1"
}

$pwd = $args[0]
$top = $args[1]
$target = $args[2]

cd $top
New-Item -ea 0 -Force -ItemType Directory "$top/bin/wcp"
New-Item -ea 0 -Force -ItemType Directory "$top/cache/wcp"
"${top}/hack/windows/setup.ps1"
nmake /P
pwd
cd "$top/cache/wcp"
pwd
cmake --debug-find -G "Visual Studio 17" "$top/ee/wcp"
cmake --build . --config Release
