from . import agent_pb2 as _agent_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class WhoAmIRequest(_message.Message):
    __slots__ = ("header",)
    HEADER_FIELD_NUMBER: _ClassVar[int]
    header: _agent_pb2.RequestHeader
    def __init__(self, header: _Optional[_Union[_agent_pb2.RequestHeader, _Mapping]] = ...) -> None: ...

class WhoAmIResponse(_message.Message):
    __slots__ = ("header", "body")
    HEADER_FIELD_NUMBER: _ClassVar[int]
    BODY_FIELD_NUMBER: _ClassVar[int]
    header: _agent_pb2.ResponseHeader
    body: str
    def __init__(self, header: _Optional[_Union[_agent_pb2.ResponseHeader, _Mapping]] = ..., body: _Optional[str] = ...) -> None: ...

class CurrentTokenRequest(_message.Message):
    __slots__ = ("header", "type", "caller_pid")
    class Type(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
        __slots__ = ()
        UNSPECIFIED: _ClassVar[CurrentTokenRequest.Type]
        VERIFIED: _ClassVar[CurrentTokenRequest.Type]
        UNVERIFIED: _ClassVar[CurrentTokenRequest.Type]
    UNSPECIFIED: CurrentTokenRequest.Type
    VERIFIED: CurrentTokenRequest.Type
    UNVERIFIED: CurrentTokenRequest.Type
    HEADER_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    CALLER_PID_FIELD_NUMBER: _ClassVar[int]
    header: _agent_pb2.RequestHeader
    type: CurrentTokenRequest.Type
    caller_pid: int
    def __init__(self, header: _Optional[_Union[_agent_pb2.RequestHeader, _Mapping]] = ..., type: _Optional[_Union[CurrentTokenRequest.Type, str]] = ..., caller_pid: _Optional[int] = ...) -> None: ...

class CurrentTokenResponse(_message.Message):
    __slots__ = ("header", "token", "raw", "url")
    HEADER_FIELD_NUMBER: _ClassVar[int]
    TOKEN_FIELD_NUMBER: _ClassVar[int]
    RAW_FIELD_NUMBER: _ClassVar[int]
    URL_FIELD_NUMBER: _ClassVar[int]
    header: _agent_pb2.ResponseHeader
    token: _agent_pb2.Token
    raw: str
    url: str
    def __init__(self, header: _Optional[_Union[_agent_pb2.ResponseHeader, _Mapping]] = ..., token: _Optional[_Union[_agent_pb2.Token, _Mapping]] = ..., raw: _Optional[str] = ..., url: _Optional[str] = ...) -> None: ...

class TokenExchangeRequest(_message.Message):
    __slots__ = ("header", "audience", "scopes", "actor_token", "actor_token_type", "requested_token_type")
    HEADER_FIELD_NUMBER: _ClassVar[int]
    AUDIENCE_FIELD_NUMBER: _ClassVar[int]
    SCOPES_FIELD_NUMBER: _ClassVar[int]
    ACTOR_TOKEN_FIELD_NUMBER: _ClassVar[int]
    ACTOR_TOKEN_TYPE_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_TOKEN_TYPE_FIELD_NUMBER: _ClassVar[int]
    header: _agent_pb2.RequestHeader
    audience: str
    scopes: _containers.RepeatedScalarFieldContainer[str]
    actor_token: str
    actor_token_type: str
    requested_token_type: str
    def __init__(self, header: _Optional[_Union[_agent_pb2.RequestHeader, _Mapping]] = ..., audience: _Optional[str] = ..., scopes: _Optional[_Iterable[str]] = ..., actor_token: _Optional[str] = ..., actor_token_type: _Optional[str] = ..., requested_token_type: _Optional[str] = ...) -> None: ...

class TokenExchangeResponse(_message.Message):
    __slots__ = ("header", "access_token", "expires_in")
    HEADER_FIELD_NUMBER: _ClassVar[int]
    ACCESS_TOKEN_FIELD_NUMBER: _ClassVar[int]
    EXPIRES_IN_FIELD_NUMBER: _ClassVar[int]
    header: _agent_pb2.ResponseHeader
    access_token: str
    expires_in: int
    def __init__(self, header: _Optional[_Union[_agent_pb2.ResponseHeader, _Mapping]] = ..., access_token: _Optional[str] = ..., expires_in: _Optional[int] = ...) -> None: ...

class AuthorizeRequest(_message.Message):
    __slots__ = ("header", "service", "uid")
    HEADER_FIELD_NUMBER: _ClassVar[int]
    SERVICE_FIELD_NUMBER: _ClassVar[int]
    UID_FIELD_NUMBER: _ClassVar[int]
    header: _agent_pb2.RequestHeader
    service: str
    uid: str
    def __init__(self, header: _Optional[_Union[_agent_pb2.RequestHeader, _Mapping]] = ..., service: _Optional[str] = ..., uid: _Optional[str] = ...) -> None: ...

class AuthorizeResponse(_message.Message):
    __slots__ = ("header",)
    HEADER_FIELD_NUMBER: _ClassVar[int]
    header: _agent_pb2.ResponseHeader
    def __init__(self, header: _Optional[_Union[_agent_pb2.ResponseHeader, _Mapping]] = ...) -> None: ...
