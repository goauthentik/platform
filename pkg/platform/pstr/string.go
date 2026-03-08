package pstr

import "runtime"

type PlatformString struct {
	Windows  *string
	Darwin   *string
	Linux    *string
	Fallback string
}

func (ps PlatformString) fallback(opts ...*string) string {
	for _, opt := range opts {
		if opt != nil {
			return *opt
		}
	}
	return ps.Fallback
}

func (ps PlatformString) ForWindows() string {
	return ps.fallback(ps.Windows, ps.Linux)
}

func (ps PlatformString) ForDarwin() string {
	return ps.fallback(ps.Darwin, ps.Linux)
}

func (ps PlatformString) ForLinux() string {
	return ps.fallback(ps.Linux)
}

func (ps PlatformString) ForCurrent() string {
	switch runtime.GOOS {
	case "windows":
		return ps.ForWindows()
	case "linux":
		return ps.ForLinux()
	case "darwin":
		return ps.ForDarwin()
	}
	return ps.Fallback
}
