Set-PSDebug -Trace 1
$ErrorActionPreference = "Stop"

$pwd = $args[0]
$top = $args[1]
$target = $args[2]

cd $top
New-Item -ea 0 -Force -ItemType Directory "$top/bin/wcp"
New-Item -ea 0 -Force -ItemType Directory "$top/cache/wcp"
. hack/windows/setup.ps1
nmake /P
pwd
cd "$top/cache/wcp"
pwd
cmake --debug-find -G "Visual Studio 17" "$top/ee/wcp"
cmake --build . --config Release
