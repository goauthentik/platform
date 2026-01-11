#include "include/ak_log.h"
#include "include/ak_version.h"
#include "spdlog/async.h"
#include "spdlog/sinks/rotating_file_sink.h"
#include "spdlog/sinks/win_eventlog_sink.h"
#include "spdlog/spdlog.h"
#include <string>

bool g_logSetup;

const auto _ak_log_max_size = 1024 * 1024 * 50;
const auto _ak_log_max_files = 3;

void ak_setup_logs(const char* logger_name) {
  const auto logger = spdlog::rotating_logger_mt(
      logger_name,
      std::string(AK_PROGRAM_DATA).append("\\logs\\").append(logger_name).append(".log").c_str(),
      _ak_log_max_size, _ak_log_max_files);
  spdlog::set_level(spdlog::level::debug);
  spdlog::flush_every(std::chrono::seconds(5));
  spdlog::set_default_logger(logger);
  SPDLOG_INFO("authentik Platform {} Version {}", logger_name, AK_VERSION);
  g_logSetup = true;
}

void ak_teardown_logs() {
  if (!g_logSetup) return;
  spdlog::shutdown();
}
