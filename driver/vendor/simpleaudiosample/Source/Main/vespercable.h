#pragma once

#include <ntddk.h>

NTSTATUS VesperCableInitialize();
VOID VesperCableCleanup();
VOID VesperCablePush(_In_reads_bytes_(ByteCount) const UCHAR* Source, _In_ ULONG ByteCount);
VOID VesperCablePop(_Out_writes_bytes_(ByteCount) UCHAR* Destination, _In_ ULONG ByteCount);
