#include "include/ak_log.h"
#include "include/ak_version.h"
#include "spdlog/async.h"
#include "spdlog/sinks/dist_sink.h"
#include "spdlog/sinks/rotating_file_sink.h"
#include "spdlog/sinks/win_eventlog_sink.h"
#include "spdlog/spdlog.h"
#include <string>

bool g_logSetup;

const auto _ak_log_max_size = 1024 * 1024 * 50;
const auto _ak_log_max_files = 3;

void ak_setup_logs(const char* logger_name) {
  const auto dist_sink = std::make_shared<spdlog::sinks::dist_sink_mt>();

  const auto file_sink = std::make_shared<spdlog::sinks::rotating_file_sink_mt>(
      std::string(AK_PROGRAM_DATA).append("\\logs\\").append(logger_name).append(".log").c_str(),
      _ak_log_max_size, _ak_log_max_files);
  const auto event_log_sink = std::make_shared<spdlog::sinks::win_eventlog_sink_mt>(logger_name);
  event_log_sink->set_pattern("[%n] [proc=%P, thread=%t] %v");

  dist_sink->add_sink(file_sink);
  dist_sink->add_sink(event_log_sink);

  const auto logger = std::make_shared<spdlog::logger>(logger_name, dist_sink);

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
