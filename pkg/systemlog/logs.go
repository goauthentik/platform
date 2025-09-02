package systemlog

func Setup(appName string) error {
	if !ShouldSwitch() {
		return nil
	}
	return ForceSetup(appName)
}
