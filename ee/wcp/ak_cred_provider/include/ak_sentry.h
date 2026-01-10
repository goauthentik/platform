#pragma once

#include <sentry.h>

void SentrySetup(const char* component);
void SentryShutdown();

static void ak_sentry_log_callback(sentry_level_t level, const char *message, va_list args, void *userdata);
