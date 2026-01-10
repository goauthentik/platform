#include "pch.h"

#include "ak_log.h"
#include "ak_version.h"
#include "spdlog/async.h"
#include "spdlog/sinks/basic_file_sink.h"
#include "spdlog/sinks/win_eventlog_sink.h"
#include "spdlog/spdlog.h"
#include <string>

bool g_logSetup;;

void SetupLogs(const char* logger_name) {
  const auto logger = spdlog::basic_logger_mt(
      logger_name,
      std::string(AK_PROGRAM_DATA).append("\\logs\\").append(logger_name).append(".log").c_str());
  spdlog::set_level(spdlog::level::debug);
  spdlog::flush_every(std::chrono::seconds(5));
  spdlog::set_default_logger(logger);
  SPDLOG_INFO("authentik Platform Credential Provider Version {}", AK_WCP_VERSION);
  g_logSetup = true;
}
