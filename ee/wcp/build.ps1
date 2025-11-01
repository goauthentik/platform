Set-PSDebug -Trace 1
cd ../..
New-Item -ea 0 -Force -ItemType Directory "bin/wcp"
New-Item -ea 0 -Force -ItemType Directory "cache/wcp"
. hack/windows/setup.ps1
nmake /P
pwd
cd "./cache/wcp"
pwd
# To avoid the OpenSSL build using linux `link.exe`
mv /usr/bin/link.exe /usr/bin/link_bak

cmake --debug-find -G "Visual Studio 17" "../../ee/wcp"
cmake --build . --config Release
cd ../..
cp "./cache/wcp/Release/*" "bin/wcp/"
