#pragma once

#include <sentry.h>

void ak_setup_sentry(const char* component);
void ak_teardown_sentry();

static void ak_sentry_log_callback(sentry_level_t level, const char* message, va_list args,
                                   void* userdata);
