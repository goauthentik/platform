#include <Windows.h>
#include <string>
#include "spdlog/async.h"
#include "spdlog/sinks/basic_file_sink.h"
#include "spdlog/sinks/win_eventlog_sink.h"
#include "spdlog/spdlog.h"

void SetupLogsPath(std::string folder,const char *logger_name) {
  const auto logger = spdlog::basic_logger_mt(logger_name, folder + "\\ak.log");
  spdlog::set_level(spdlog::level::debug);
  spdlog::flush_every(std::chrono::seconds(1));
  spdlog::set_default_logger(logger);
}

STDAPI_(BOOL)
DllMain(__in HINSTANCE hinstDll, __in DWORD dwReason, __in LPVOID lpReserved) {
  switch (dwReason) {
  case DLL_PROCESS_ATTACH:
    SetupLogsPath("C:", "ak_lsa");
    spdlog::debug("DllMain::DLL_PROCESS_ATTACH");
    break;
  case DLL_PROCESS_DETACH:
    spdlog::debug("DllMain::DLL_PROCESS_DETACH");
    spdlog::shutdown();
    break;
  }
  return TRUE;
}
