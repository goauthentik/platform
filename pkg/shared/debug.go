package shared

import (
	"net/http"
	"net/http/httptest"
	"net/http/pprof"

	log "github.com/sirupsen/logrus"
)

var debugServer *httptest.Server

func startDebugServer(l *log.Entry) {
	h := &http.ServeMux{}
	h.HandleFunc("/debug/pprof/", pprof.Index)
	h.HandleFunc("/debug/pprof/cmdline", pprof.Cmdline)
	h.HandleFunc("/debug/pprof/profile", pprof.Profile)
	h.HandleFunc("/debug/pprof/symbol", pprof.Symbol)
	h.HandleFunc("/debug/pprof/trace", pprof.Trace)
	debugServer = httptest.NewServer(h)
	l.WithField("on", debugServer.URL).Debug("Started debug server")
}
