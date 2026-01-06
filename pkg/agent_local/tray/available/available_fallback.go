//go:build !(linux || freebsd || openbsd || netbsd)

package available

func SystrayAvailable() bool {
	return true
}
