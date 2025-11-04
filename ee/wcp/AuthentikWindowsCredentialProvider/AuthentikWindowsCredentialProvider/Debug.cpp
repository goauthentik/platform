#include "pch.h"

#include "Debug.h"
#include "spdlog/async.h"
#include "spdlog/sinks/basic_file_sink.h"
#include "spdlog/sinks/win_eventlog_sink.h"
#include "spdlog/spdlog.h"
#include <string>
#define BUFFER_SIZE 10000

std::mutex g_dbgMutex;
bool g_logSetup;
extern std::string g_strPath;

void SetupLogs(const char *logger_name) {
  // const auto win_sink =
  // std::make_shared<spdlog::sinks::win_eventlog_sink_mt>("authentik WCP");
  // const auto file_sink =
  // std::make_shared<spdlog::sinks::basic_file_sink_mt>(g_strPath+"\\wcp.log");
  // std::vector<spdlog::sink_ptr> sinks {win_sink, file_sink};

  // const auto logger = std::make_shared<spdlog::async_logger>("wcp",
  // sinks.begin(), sinks.end(), spdlog::thread_pool(),
  // spdlog::async_overflow_policy::block);
  const auto logger =
      spdlog::basic_logger_mt(logger_name, g_strPath + "\\wcp.log");
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
