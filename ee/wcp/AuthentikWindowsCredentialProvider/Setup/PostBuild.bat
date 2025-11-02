:: BISMILLAAHIRRAHMAANIRRAHEEM

:: copy %0\..\..\..\x64\Debug\AuthentikWindowsCredentialProvider.dll %userprofile%\Desktop
:: signtool sign /v /s PrivateCertStore /n Contoso.com(Test) /t http://timestamp.digicert.com /fd SHA256 %userprofile%\Desktop\AuthentikWindowsCredentialProvider.dll
:: signtool verify /pa %userprofile%\Desktop\AuthentikWindowsCredentialProvider.dll
