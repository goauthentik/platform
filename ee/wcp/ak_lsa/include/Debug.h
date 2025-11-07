// BISMILLAAHIRRAHMAANIRRAHEEM

#pragma once
#include <string>
#include <thread>

void Debug(const char* data, ...);
void SetupLogs(const char* logger_name);
void SetupLogsPath(std::string folder, const char *logger_name);
