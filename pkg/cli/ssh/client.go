package ssh

import (
	"fmt"
	"net"
	"os"
	"os/user"
	"strconv"
	"strings"

	"github.com/google/uuid"
	"github.com/pkg/errors"
	log "github.com/sirupsen/logrus"
	"github.com/skeema/knownhosts"
	"goauthentik.io/platform/pkg/agent_local/client"
	"golang.org/x/crypto/ssh"
)

type SSHClient struct {
	host string
	port int
	user string

	Command      string
	Insecure     bool
	AgentClient  *client.AgentClient
	AgentProfile string

	agentToken string

	knownHosts       *knownhosts.HostKeyDB
	knownHostsFile   string
	remoteSocketPath string

	log *log.Entry
}

func New(host string, port int, user string) (*SSHClient, error) {
	c := &SSHClient{
		log:  log.WithField("component", "ssh"),
		host: host,
		port: port,
		user: user,
	}

	khf, err := DefaultKnownHostsPath()
	if err != nil {
		return nil, errors.Wrap(err, "failed to locate known_hosts")
	}
	if _, err := os.Stat(khf); os.IsNotExist(err) {
		_, err := os.OpenFile(khf, os.O_CREATE, 0600)
		if err != nil {
			return nil, errors.Wrap(err, "failed to create known_hosts file")
		}
	}
	c.knownHostsFile = khf
	kh, err := knownhosts.NewDB(khf)
	if err != nil {
		return nil, errors.Wrap(err, "failed to read known_hosts")
	}
	c.knownHosts = kh

	uid := uuid.New().String()
	c.remoteSocketPath = fmt.Sprintf("/var/run/authentik/agent-%s.sock", uid)

	return c, nil
}

func ParseArgs(args []string) (*SSHClient, error) {
	u, err := user.Current()
	if err != nil {
		return nil, err
	}
	if len(args) < 1 {
		return nil, errors.New("missing host")
	}
	host := args[0]
	user := u.Username
	port := "22"
	if strings.Contains(host, "@") {
		_parts := strings.Split(host, "@")
		user = _parts[0]
		host = _parts[1]
	}
	if strings.Contains(host, ":") {
		_parts := strings.Split(host, ":")
		host = _parts[0]
		port = _parts[1]
	}
	nport, err := strconv.Atoi(port)
	if err != nil {
		return nil, err
	}
	cmd := ""
	if len(args) > 1 {
		cmd = args[1]
	}
	c, err := New(host, nport, user)
	if err != nil {
		return nil, err
	}
	c.Command = cmd
	return c, nil
}

func (c *SSHClient) Connect() error {
	client, err := ssh.Dial("tcp", net.JoinHostPort(c.host, strconv.Itoa(c.port)), c.getConfig())
	if err != nil {
		return err
	}
	defer func() {
		err := client.Close()
		if err != nil && !errors.Is(err, net.ErrClosed) {
			c.log.WithError(err).Warning("Failed to close client")
		}
	}()

	go func() {
		err := c.ForwardAgentSocket(c.remoteSocketPath, client)
		if err != nil {
			fmt.Printf("Warning: %v\n", err.Error())
		}
	}()
	if c.Command != "" {
		return c.command(client)
	}
	return c.shell(client)
}
