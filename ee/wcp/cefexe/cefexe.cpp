#include "include/cef_app.h"
#include "cefsimple/simple_app.h"

#include "AuthentikWindowsCredentialProvider/AuthentikWindowsCredentialProvider/include/Debug.h"
#include <Synchapi.h>
//#pragma comment(lib,"libcef.lib")

// int main(int argc, char* argv[])
// {

int APIENTRY wWinMain(HINSTANCE hInstance,
                      HINSTANCE hPrevInstance,
                      LPTSTR lpCmdLine,
                      int nCmdShow) {
  UNREFERENCED_PARAMETER(hPrevInstance);
  UNREFERENCED_PARAMETER(lpCmdLine);


	HINSTANCE hInst = GetModuleHandle(0);

	CefMainArgs main_args(hInst);
	// Debug("cefexe");

	// printf("> %d\n", argc);
	// for (int i = 0; i < argc; ++i)
	// {
	// 	const wchar_t str[100] = { i, '\0' };
	// 	const wchar_t txt[1000] = { *argv[i], '\0'};
	// 	printf("%s\n", argv[i]);
	// 	const char str1[100] = { i, '\0' };
	// 	const char txt1[1000] = { *argv[i], '\0' };
	// 	// Debug(str1);
	// 	// Debug(txt1);
	// }

	//Sleep(3000);

	int ret = 0;
	try
	{
		// Debug("cefexe: CefExecuteProcess");
		ret = CefExecuteProcess(main_args, nullptr, nullptr);
		// Debug("cefexe: CefExecuteProcess... done");
	}
	catch (const std::exception&)
	{
		Debug("cefexe: CefExecuteProcess... catch...!");
	}

	return ret;
}