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
		cl := l.WithFields(f)

		p, ok := peer.FromContext(ctx)
		if ok && p.AuthInfo != nil {
			if ga, ok := p.AuthInfo.(grpc_creds.AuthInfo); ok && ga.Creds != nil {
				cl = l.WithField("auth.pid", ga.Creds.PID)
				l.WithFields(log.Fields{
					"proc":   ga.Creds.Proc.String(),
					"parent": ga.Creds.Parent.String(),
					"pid":    ga.Creds.PID,
					"uid":    ga.Creds.UID,
					"gid":    ga.Creds.GID,
				}).Debug("GRPC auth info debug")
			}
		}

		switch lvl {
		case logging.LevelDebug:
			cl.Debug(msg)
		case logging.LevelInfo:
			cl.Info(msg)
		case logging.LevelWarn:
			cl.Warn(msg)
		case logging.LevelError:
			cl.Error(msg)
		default:
			panic(fmt.Sprintf("unknown level %v", lvl))
		}
	})
}
