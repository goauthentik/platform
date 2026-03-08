package ssh

import (
	"io"
	"net"
	"strings"

	"goauthentik.io/platform/pkg/agent_local/types"
	"goauthentik.io/platform/pkg/platform/socket"
	"golang.org/x/crypto/ssh"
)

func (c *SSHClient) ForwardAgentSocket(remoteSocket string, client *ssh.Client) error {
	localSocket := types.GetAgentSocketPath()
	remoteListener, err := client.Listen("unix", remoteSocket)
	if err != nil {
		return err
	}
	defer func() {
		err := remoteListener.Close()
		if err != nil {
			c.log.WithError(err).Warning("Failed to close remote listener")
		}
	}()
	c.log.Debugf("remote listening %s → local %s", remoteSocket, localSocket.ForCurrent())

	for {
		remoteConn, err := remoteListener.Accept()
		if err != nil {
			c.log.WithError(err).Debug("remote Accept error")
			continue
		}
		go func(rc net.Conn) {
			defer func() {
				err := rc.Close()
				if err != nil {
					c.log.WithError(err).Warning("failed to close remote connection")
				}
			}()
			// Dial the local unix socket
			lc, err := socket.Connect(localSocket)
			if err != nil {
				c.log.WithError(err).Debugf("local dial %s failed", localSocket.ForCurrent())
				return
			}
			defer func() {
				err := lc.Close()
				if err != nil {
					c.log.WithError(err).Warning("failed to close local connection")
				}
			}()

			done := make(chan struct{}, 2)
			go func() {
				_, err := io.Copy(rc, lc)
				if err != nil && !strings.Contains(err.Error(), "use of closed network connection") {
					c.log.WithError(err).Warning("failed to copy from remote to local")
				}
				done <- struct{}{}
			}()
			go func() {
				_, err := io.Copy(lc, rc)
				if err != nil {
					c.log.WithError(err).Warning("failed to copy from local to remote")
				}
				done <- struct{}{}
			}()
			<-done
		}(remoteConn)
	}
}
