package shared

import (
	"slices"

	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/logging"
	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/recovery"
	grpc_sentry "github.com/johnbellone/grpc-middleware-sentry"
	log "github.com/sirupsen/logrus"
	systemlog "goauthentik.io/platform/pkg/platform/log"

	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func CommonGRPCServerOpts(l *log.Entry, extra ...grpc.ServerOption) []grpc.ServerOption {
	allowedErrorCodes := []codes.Code{
		codes.Internal,
		codes.Unimplemented,
		codes.Unknown,
	}
	sentryOpts := []grpc_sentry.Option{
		grpc_sentry.WithRepanicOption(true),
		grpc_sentry.WithReportOn(func(err error) bool {
			st, ok := status.FromError(err)
			if ok {
				return slices.Contains(allowedErrorCodes, st.Code())
			}
			return true
		}),
	}
	recoveryOpts := []recovery.Option{
		recovery.WithRecoveryHandler(func(p any) (err error) {
			if e, ok := p.(error); ok {
				l.WithError(e).Warning("GRPC method panic'd")
			} else {
				l.WithField("p", p).Warning("GRPC method panic'd")
			}
			return status.Errorf(codes.Unknown, "panic triggered")
		}),
	}
	opts := []grpc.ServerOption{
		grpc.ChainUnaryInterceptor(
			logging.UnaryServerInterceptor(systemlog.InterceptorLogger(l)),
			grpc_sentry.UnaryServerInterceptor(sentryOpts...),
			recovery.UnaryServerInterceptor(recoveryOpts...),
		),
		grpc.ChainStreamInterceptor(
			logging.StreamServerInterceptor(systemlog.InterceptorLogger(l)),
			grpc_sentry.StreamServerInterceptor(sentryOpts...),
			recovery.StreamServerInterceptor(recoveryOpts...),
		),
	}
	return append(opts, extra...)
}
