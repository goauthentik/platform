import datetime

from . import agent_pb2 as _agent_pb2
from google.protobuf import timestamp_pb2 as _timestamp_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class CacheStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    UNSPECIFIED: _ClassVar[CacheStatus]
    VALID: _ClassVar[CacheStatus]
    NOT_FOUND: _ClassVar[CacheStatus]
    EXPIRED: _ClassVar[CacheStatus]
UNSPECIFIED: CacheStatus
VALID: CacheStatus
NOT_FOUND: CacheStatus
EXPIRED: CacheStatus

class CacheGetRequest(_message.Message):
    __slots__ = ("header", "keys")
    HEADER_FIELD_NUMBER: _ClassVar[int]
    KEYS_FIELD_NUMBER: _ClassVar[int]
    header: _agent_pb2.RequestHeader
    keys: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, header: _Optional[_Union[_agent_pb2.RequestHeader, _Mapping]] = ..., keys: _Optional[_Iterable[str]] = ...) -> None: ...

class CacheGetResponse(_message.Message):
    __slots__ = ("header", "status", "expiry", "value")
    HEADER_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    EXPIRY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    header: _agent_pb2.ResponseHeader
    status: CacheStatus
    expiry: _timestamp_pb2.Timestamp
    value: str
    def __init__(self, header: _Optional[_Union[_agent_pb2.ResponseHeader, _Mapping]] = ..., status: _Optional[_Union[CacheStatus, str]] = ..., expiry: _Optional[_Union[datetime.datetime, _timestamp_pb2.Timestamp, _Mapping]] = ..., value: _Optional[str] = ...) -> None: ...

class CacheSetRequest(_message.Message):
    __slots__ = ("header", "keys", "expiry", "value")
    HEADER_FIELD_NUMBER: _ClassVar[int]
    KEYS_FIELD_NUMBER: _ClassVar[int]
    EXPIRY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    header: _agent_pb2.RequestHeader
    keys: _containers.RepeatedScalarFieldContainer[str]
    expiry: _timestamp_pb2.Timestamp
    value: str
    def __init__(self, header: _Optional[_Union[_agent_pb2.RequestHeader, _Mapping]] = ..., keys: _Optional[_Iterable[str]] = ..., expiry: _Optional[_Union[datetime.datetime, _timestamp_pb2.Timestamp, _Mapping]] = ..., value: _Optional[str] = ...) -> None: ...

class CacheSetResponse(_message.Message):
    __slots__ = ("header",)
    HEADER_FIELD_NUMBER: _ClassVar[int]
    header: _agent_pb2.ResponseHeader
    def __init__(self, header: _Optional[_Union[_agent_pb2.ResponseHeader, _Mapping]] = ...) -> None: ...
