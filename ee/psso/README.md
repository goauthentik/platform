# macOS PSSO

Check if AppEx is loaded: `pluginkit -m --raw | grep io.goauthentik`
Load AppEx: `pluginkit -a ./<path to>/PSSO.appex`
Unload AppEx: `pluginkit -r ./<path to>/PSSO.appex`

Check logs:

log show --debug --info --predicate 'subsystem contains "io.goauthentik.platform"'
log stream --debug --info --predicate 'subsystem contains "io.goauthentik.platform"'
log show --predicate 'subsystem == "com.apple.AppSSO"'
log stream --predicate 'subsystem == "com.apple.AppSSO"'
log stream --info --predicate 'subsystem contains "com.apple.AppSSO" OR process contains "AppSSO"'
log stream --debug --info  --predicate 'subsystem contains "com.apple.AppSSO" OR process contains "AppSSO"'
