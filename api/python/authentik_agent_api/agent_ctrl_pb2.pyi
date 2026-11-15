import datetime

from . import agent_pb2 as _agent_pb2
from google.protobuf import empty_pb2 as _empty_pb2
from google.protobuf import timestamp_pb2 as _timestamp_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class ProfileStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    UNSPECIFIED: _ClassVar[ProfileStatus]
    ACTIVE: _ClassVar[ProfileStatus]
    FAILED: _ClassVar[ProfileStatus]
UNSPECIFIED: ProfileStatus
ACTIVE: ProfileStatus
FAILED: ProfileStatus

class SetupRequest(_message.Message):
    __slots__ = ("header", "authentik_url", "app_slug", "client_id", "access_token", "refresh_token", "dpop_private_key")
    HEADER_FIELD_NUMBER: _ClassVar[int]
    AUTHENTIK_URL_FIELD_NUMBER: _ClassVar[int]
    APP_SLUG_FIELD_NUMBER: _ClassVar[int]
    CLIENT_ID_FIELD_NUMBER: _ClassVar[int]
    ACCESS_TOKEN_FIELD_NUMBER: _ClassVar[int]
    REFRESH_TOKEN_FIELD_NUMBER: _ClassVar[int]
    DPOP_PRIVATE_KEY_FIELD_NUMBER: _ClassVar[int]
    header: _agent_pb2.RequestHeader
    authentik_url: str
    app_slug: str
    client_id: str
    access_token: str
    refresh_token: str
    dpop_private_key: str
    def __init__(self, header: _Optional[_Union[_agent_pb2.RequestHeader, _Mapping]] = ..., authentik_url: _Optional[str] = ..., app_slug: _Optional[str] = ..., client_id: _Optional[str] = ..., access_token: _Optional[str] = ..., refresh_token: _Optional[str] = ..., dpop_private_key: _Optional[str] = ...) -> None: ...

class SetupResponse(_message.Message):
    __slots__ = ("header",)
    HEADER_FIELD_NUMBER: _ClassVar[int]
    header: _agent_pb2.ResponseHeader
    def __init__(self, header: _Optional[_Union[_agent_pb2.ResponseHeader, _Mapping]] = ...) -> None: ...

class Profile(_message.Message):
    __slots__ = ("name", "username", "authentik_url", "last_renewed", "next_renew", "status", "dpop_bound")
    NAME_FIELD_NUMBER: _ClassVar[int]
    USERNAME_FIELD_NUMBER: _ClassVar[int]
    AUTHENTIK_URL_FIELD_NUMBER: _ClassVar[int]
    LAST_RENEWED_FIELD_NUMBER: _ClassVar[int]
    NEXT_RENEW_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DPOP_BOUND_FIELD_NUMBER: _ClassVar[int]
    name: str
    username: str
    authentik_url: str
    last_renewed: _timestamp_pb2.Timestamp
    next_renew: _timestamp_pb2.Timestamp
    status: ProfileStatus
    dpop_bound: bool
    def __init__(self, name: _Optional[str] = ..., username: _Optional[str] = ..., authentik_url: _Optional[str] = ..., last_renewed: _Optional[_Union[datetime.datetime, _timestamp_pb2.Timestamp, _Mapping]] = ..., next_renew: _Optional[_Union[datetime.datetime, _timestamp_pb2.Timestamp, _Mapping]] = ..., status: _Optional[_Union[ProfileStatus, str]] = ..., dpop_bound: _Optional[bool] = ...) -> None: ...

class ListProfilesResponse(_message.Message):
    __slots__ = ("header", "profiles")
    HEADER_FIELD_NUMBER: _ClassVar[int]
    PROFILES_FIELD_NUMBER: _ClassVar[int]
    header: _agent_pb2.ResponseHeader
    profiles: _containers.RepeatedCompositeFieldContainer[Profile]
    def __init__(self, header: _Optional[_Union[_agent_pb2.ResponseHeader, _Mapping]] = ..., profiles: _Optional[_Iterable[_Union[Profile, _Mapping]]] = ...) -> None: ...

class CurrentProfileResponse(_message.Message):
    __slots__ = ("header", "profile")
    HEADER_FIELD_NUMBER: _ClassVar[int]
    PROFILE_FIELD_NUMBER: _ClassVar[int]
    header: _agent_pb2.ResponseHeader
    profile: str
    def __init__(self, header: _Optional[_Union[_agent_pb2.ResponseHeader, _Mapping]] = ..., profile: _Optional[str] = ...) -> None: ...
