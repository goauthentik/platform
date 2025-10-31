Set-PSDebug -Trace 1
cd ../..
New-Item -ea 0 -Force -ItemType Directory "bin/wcp"
. hack/windows/setup.ps1
nmake -P
pwd
cd "./bin/wcp"
pwd
cmake --debug-find -G "Visual Studio 17" "../../ee/wcp"
cmake --build . --config Release
