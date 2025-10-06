# macOS PSSO

Check if AppEx is loaded: `pluginkit -m --raw | grep io.goauthentik`
Load AppEx: `pluginkit -a ./<path to>/PSSO.appex`
Unload AppEx: `pluginkit -r ./<path to>/PSSO.appex`
