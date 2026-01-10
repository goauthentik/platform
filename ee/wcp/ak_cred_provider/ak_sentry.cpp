#include "ak_version.h"
#include <string>

#define SENTRY_BUILD_STATIC 1
#include <sentry.h>
#include <spdlog/spdlog.h>

bool g_sentrySetup;

static void ak_sentry_log_callback(sentry_level_t level, const char *message, va_list args, void *userdata) {
  (void)level;
  (void)userdata;
  char formatted_message[1024];
  vsnprintf(formatted_message, sizeof(formatted_message), message, args);

  spdlog::get("sentry")->debug(formatted_message);
}

void SentrySetup(const char *component) {
  if (g_sentrySetup) return;
  spdlog::register_logger(spdlog::default_logger()->clone("sentry"));

  std::string release = std::string("ak-platform-wcp-")
                            .append(component)
                            .append("@")
                            .append(AK_WCP_VERSION);
  sentry_options_t *options = sentry_options_new();
  sentry_options_set_database_path(options, std::string(AK_PROGRAM_DATA).append("\\wcp-sentry\\").c_str());
  sentry_options_set_debug(options, 1);
  sentry_options_set_logger(options, ak_sentry_log_callback, NULL);
  sentry_options_set_dsn(options,
                         "https://"
                         "c83cdbb55c9bd568ecfa275932b6de17@o4504163616882688."
                         "ingest.us.sentry.io/4509208005312512");
  sentry_options_set_release(options, release.c_str());
  sentry_init(options);
  spdlog::debug("Sentry initialized");
  g_sentrySetup = true;
}

void SentryShutdown() { sentry_shutdown(); }
