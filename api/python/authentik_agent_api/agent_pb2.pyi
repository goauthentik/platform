import datetime

from google.protobuf import timestamp_pb2 as _timestamp_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class RequestHeader(_message.Message):
    __slots__ = ("profile",)
    PROFILE_FIELD_NUMBER: _ClassVar[int]
    profile: str
    def __init__(self, profile: _Optional[str] = ...) -> None: ...

class ResponseHeader(_message.Message):
    __slots__ = ("successful",)
    SUCCESSFUL_FIELD_NUMBER: _ClassVar[int]
    successful: bool
    def __init__(self, successful: _Optional[bool] = ...) -> None: ...

class Token(_message.Message):
    __slots__ = ("preferred_username", "iss", "sub", "aud", "exp", "nbf", "iat", "jti")
    PREFERRED_USERNAME_FIELD_NUMBER: _ClassVar[int]
    ISS_FIELD_NUMBER: _ClassVar[int]
    SUB_FIELD_NUMBER: _ClassVar[int]
    AUD_FIELD_NUMBER: _ClassVar[int]
    EXP_FIELD_NUMBER: _ClassVar[int]
    NBF_FIELD_NUMBER: _ClassVar[int]
    IAT_FIELD_NUMBER: _ClassVar[int]
    JTI_FIELD_NUMBER: _ClassVar[int]
    preferred_username: str
    iss: str
    sub: str
    aud: _containers.RepeatedScalarFieldContainer[str]
    exp: _timestamp_pb2.Timestamp
    nbf: _timestamp_pb2.Timestamp
    iat: _timestamp_pb2.Timestamp
    jti: str
    def __init__(self, preferred_username: _Optional[str] = ..., iss: _Optional[str] = ..., sub: _Optional[str] = ..., aud: _Optional[_Iterable[str]] = ..., exp: _Optional[_Union[datetime.datetime, _timestamp_pb2.Timestamp, _Mapping]] = ..., nbf: _Optional[_Union[datetime.datetime, _timestamp_pb2.Timestamp, _Mapping]] = ..., iat: _Optional[_Union[datetime.datetime, _timestamp_pb2.Timestamp, _Mapping]] = ..., jti: _Optional[str] = ...) -> None: ...
