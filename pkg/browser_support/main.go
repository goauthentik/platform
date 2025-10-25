package browsersupport

import (
	"context"
	"os"
	"os/signal"
	"syscall"

	systemlog "goauthentik.io/platform/pkg/platform/log"
)

func Main() {
	_ = systemlog.Setup("browser-support")
	defer systemlog.Cleanup()

	bs, err := New()
	if err != nil {
		systemlog.Get().WithError(err).Warning("failed to start browser support")
		return
	}
	ctx, cancel := context.WithCancel(context.Background())
	go func() {
		sigChan := make(chan os.Signal, 1)
		signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
		<-sigChan
		cancel()
		bs.Stop()
	}()
	bs.Start(ctx)
}
