package main

import (
	"fmt"

	"goauthentik.io/platform/pkg/pb"
	sshagent "goauthentik.io/platform/pkg/ssh_agent"
	"golang.org/x/crypto/ssh"
	"google.golang.org/protobuf/proto"
)

func main() {
	m, err := proto.Marshal(&pb.WhoAmIRequest{
		Header: &pb.RequestHeader{
			Profile: "default",
		},
	})
	if err != nil {
		panic(err)
	}
	a := ssh.Marshal(sshagent.ExtAuthentikAgentTunnelData{
		Method: "agent_auth.AgentAuth/WhoAmI",
		Data:   m,
	})
	fmt.Printf("%+v\n", a)
}
