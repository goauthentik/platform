from google.protobuf import empty_pb2 as _empty_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class PingResponse(_message.Message):
    __slots__ = ("component", "version", "server_version")
    COMPONENT_FIELD_NUMBER: _ClassVar[int]
    VERSION_FIELD_NUMBER: _ClassVar[int]
    SERVER_VERSION_FIELD_NUMBER: _ClassVar[int]
    component: str
    version: str
    server_version: str
    def __init__(self, component: _Optional[str] = ..., version: _Optional[str] = ..., server_version: _Optional[str] = ...) -> None: ...

class CapabilitiesResponse(_message.Message):
    __slots__ = ("capabilities",)
    class Capability(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
        __slots__ = ()
        UNSPECIFIED: _ClassVar[CapabilitiesResponse.Capability]
        AUTH_INTERACTIVE: _ClassVar[CapabilitiesResponse.Capability]
        AUTH_AUTHZ: _ClassVar[CapabilitiesResponse.Capability]
        DEBUG: _ClassVar[CapabilitiesResponse.Capability]
    UNSPECIFIED: CapabilitiesResponse.Capability
    AUTH_INTERACTIVE: CapabilitiesResponse.Capability
    AUTH_AUTHZ: CapabilitiesResponse.Capability
    DEBUG: CapabilitiesResponse.Capability
    CAPABILITIES_FIELD_NUMBER: _ClassVar[int]
    capabilities: _containers.RepeatedScalarFieldContainer[CapabilitiesResponse.Capability]
    def __init__(self, capabilities: _Optional[_Iterable[_Union[CapabilitiesResponse.Capability, str]]] = ...) -> None: ...
