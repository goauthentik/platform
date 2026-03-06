#include "cefsimple/simple_app.h"
#include "include/cef_app.h"

#include "ak_common/include/ak_log.h"
#include "ak_common/include/ak_sentry.h"
#include "spdlog/spdlog.h"
#include <Synchapi.h>

int APIENTRY wWinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPTSTR lpCmdLine,
                      int nCmdShow) {
  UNREFERENCED_PARAMETER(hPrevInstance);
  UNREFERENCED_PARAMETER(lpCmdLine);

  HINSTANCE hInst = GetModuleHandle(0);

  CefMainArgs main_args(hInst);

  ak_setup_logs("ak_cefexe");
  ak_setup_sentry("cefexe");
  SPDLOG_DEBUG("wWinMain");

  int ret = 0;
  try {
    SPDLOG_DEBUG("CefExecuteProcess");
    ret = CefExecuteProcess(main_args, nullptr, nullptr);
    SPDLOG_DEBUG("CefExecuteProcess... done");
  } catch (const std::exception&) {
    SPDLOG_DEBUG("CefExecuteProcess... catch...!");
  }
  ak_teardown_sentry();
  ak_teardown_logs();
  return ret;
}
