#include "cefsimple/simple_app.h"
#include "include/cef_app.h"

#include "ak_cred_provider/include/Debug.h"
#include "ak_cred_provider/include/ak_sentry.h"
#include "spdlog/spdlog.h"
#include <Synchapi.h>
// #pragma comment(lib,"libcef.lib")

// int main(int argc, char* argv[])
// {

int APIENTRY wWinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance,
                      LPTSTR lpCmdLine, int nCmdShow) {
  UNREFERENCED_PARAMETER(hPrevInstance);
  UNREFERENCED_PARAMETER(lpCmdLine);

  HINSTANCE hInst = GetModuleHandle(0);

  CefMainArgs main_args(hInst);

  SetupLogs("cefexe");
  SentrySetup("cefexe");
  SPDLOG_DEBUG("wWinMain");

  // printf("> %d\n", argc);
  // for (int i = 0; i < argc; ++i)
  // {
  // 	const wchar_t str[100] = { i, '\0' };
  // 	const wchar_t txt[1000] = { *argv[i], '\0'};
  // 	printf("%s\n", argv[i]);
  // 	const char str1[100] = { i, '\0' };
  // 	const char txt1[1000] = { *argv[i], '\0' };
  // 	// SPDLOG_DEBUG(str1);
  // 	// SPDLOG_DEBUG(txt1);
  // }

  // Sleep(3000);

  int ret = 0;
  try {
    SPDLOG_DEBUG("CefExecuteProcess");
    ret = CefExecuteProcess(main_args, nullptr, nullptr);
    SPDLOG_DEBUG("CefExecuteProcess... done");
  } catch (const std::exception &) {
    SPDLOG_DEBUG("CefExecuteProcess... catch...!");
  }
  // spdlog::shutdown();
  SentryShutdown();
  return ret;
}
