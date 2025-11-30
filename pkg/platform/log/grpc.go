package log

import (
	"context"
	"fmt"

	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/logging"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/platform/grpc_creds"
	"google.golang.org/grpc/peer"
)

func InterceptorLogger(l log.FieldLogger) logging.Logger {
	return logging.LoggerFunc(func(ctx context.Context, lvl logging.Level, msg string, fields ...any) {
		f := make(map[string]any, len(fields)/2)
		i := logging.Fields(fields).Iterator()
		for i.Next() {
			k, v := i.At()
			f[k] = v
		}
		l := l.WithFields(f)

		p, ok := peer.FromContext(ctx)
		if ok && p.AuthInfo != nil {
			if ga, ok := p.AuthInfo.(grpc_creds.AuthInfo); ok && ga.Creds != nil {
				l = l.WithField("auth.pid", ga.Creds.PID)
				l.WithFields(log.Fields{
					"proc":           ga.Creds.Proc,
					"parent":         ga.Creds.Parent,
					"parent_exe":     ga.Creds.ParentExe,
					"parent_cmdline": ga.Creds.ParentCmdline,
					"pid":            ga.Creds.PID,
					"uid":            ga.Creds.UID,
					"gid":            ga.Creds.GID,
				}).Debug("GRPC auth info debug")
			}
		}

		switch lvl {
		case logging.LevelDebug:
			l.Debug(msg)
		case logging.LevelInfo:
			l.Info(msg)
		case logging.LevelWarn:
			l.Warn(msg)
		case logging.LevelError:
			l.Error(msg)
		default:
			panic(fmt.Sprintf("unknown level %v", lvl))
		}
	})
}
