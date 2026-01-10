#include "pch.h"

#include "Debug.h"
#include "spdlog/async.h"
#include "spdlog/sinks/basic_file_sink.h"
#include "spdlog/sinks/win_eventlog_sink.h"
#include "spdlog/spdlog.h"
#include "ak_version.h"
#include <string>
#define BUFFER_SIZE 10000

std::mutex g_dbgMutex;
bool g_logSetup;
extern std::string g_strPath;

void SetupLogs(const char *logger_name) {
  const auto logger =
      spdlog::basic_logger_mt(logger_name, std::string(AK_PROGRAM_DATA).
              append("\\logs\\").
              append(logger_name).
              append(".log").c_str());
  spdlog::set_level(spdlog::level::debug);
  spdlog::flush_every(std::chrono::seconds(5));
  spdlog::set_default_logger(logger);
  g_logSetup = true;
}

void Debug(const char *data, bool bReset) {
  g_dbgMutex.lock();
  if (!g_logSetup) {
    SetupLogs("authentik-wcp");
  }

  spdlog::debug(data);
  g_dbgMutex.unlock();
}
