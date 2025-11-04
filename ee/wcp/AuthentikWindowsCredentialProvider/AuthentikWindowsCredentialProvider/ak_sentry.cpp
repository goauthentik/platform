#include "ak_version.h"
#include <string>

#define SENTRY_BUILD_STATIC 1
#include <sentry.h>
#include <spdlog/spdlog.h>

void SentrySetup(const char *component) {
  std::string release = std::string("ak-platform-wcp-")
                            .append(component)
                            .append("@")
                            .append(AK_WCP_VERSION);
  sentry_options_t *options = sentry_options_new();
  sentry_options_set_dsn(options,
                         "https://"
                         "c83cdbb55c9bd568ecfa275932b6de17@o4504163616882688."
                         "ingest.us.sentry.io/4509208005312512");
  sentry_options_set_release(options, release.c_str());
  sentry_init(options);
  spdlog::debug("Sentry initialized");
}

void SentryShutdown() { sentry_shutdown(); }
