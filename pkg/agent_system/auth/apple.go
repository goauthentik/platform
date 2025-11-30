package auth

import (
	"context"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/pb"
)

func (auth *Server) RegisterUser(ctx context.Context, req *pb.RegisterUserRequest) (*pb.RegisterUserResponse, error) {
	ac, err := auth.dom.APIClient()
	if err != nil {
		return nil, err
	}
	u, hr, err := ac.EndpointsApi.EndpointsAgentsPssoRegisterUserCreate(ctx).AgentPSSOUserRegistrationRequest(api.AgentPSSOUserRegistrationRequest{
		UserAuth:             req.UserAuth,
		EnclaveKeyId:         req.EnclaveKeyId,
		UserSecureEnclaveKey: req.UserSecureEnclaveKey,
	}).Execute()
	if err != nil {
		auth.log.WithError(ak.HTTPToError(hr, err)).Warning("failed to register user")
		return nil, err
	}
	return &pb.RegisterUserResponse{
		Username: u.Username,
	}, nil
}

func (auth *Server) RegisterDevice(ctx context.Context, req *pb.RegisterDeviceRequest) (*pb.RegisterDeviceResponse, error) {
	ac, err := auth.dom.APIClient()
	if err != nil {
		return nil, err
	}
	d, hr, err := ac.EndpointsApi.EndpointsAgentsPssoRegisterDeviceCreate(ctx).AgentPSSODeviceRegistrationRequest(api.AgentPSSODeviceRegistrationRequest{
		DeviceSigningKey:    req.DeviceSigningKey,
		DeviceEncryptionKey: req.DeviceEncryptionKey,
		EncKeyId:            req.EncKeyId,
		SignKeyId:           req.SignKeyId,
	}).Execute()
	if err != nil {
		auth.log.WithError(ak.HTTPToError(hr, err)).Warning("failed to register device")
		return nil, err
	}
	return &pb.RegisterDeviceResponse{
		ClientId:      d.ClientId,
		Issuer:        d.Issuer,
		TokenEndpoint: d.TokenEndpoint,
		JwksEndpoint:  d.JwksEndpoint,
		Audience:      d.Audience,
		NonceEndpoint: d.NonceEndpoint,
		DeviceToken:   auth.dom.Token,
	}, nil
}
