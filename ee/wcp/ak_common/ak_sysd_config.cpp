#include "authentik_sys_bridge/ffi.h"
#include "rust/cxx.h"

bool g_configSetup;
AgentConfig g_config;

AgentConfig ak_get_config() {
    if (g_configSetup) {
        return g_config;
    }
    if (ak_sys_config(g_config)) {
        g_configSetup = true;
    }
}
