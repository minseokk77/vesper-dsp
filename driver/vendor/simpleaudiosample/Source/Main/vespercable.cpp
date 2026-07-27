#include "vespercable.h"

#define VESPER_CABLE_POOLTAG 'CpsV'
#define VESPER_CABLE_CAPACITY_BYTES (12 * 1024 * 1024)

namespace
{
    UCHAR* g_Buffer = nullptr;
    ULONG g_Capacity = 0;
    ULONGLONG g_ReadCursor = 0;
    ULONGLONG g_WriteCursor = 0;
    KSPIN_LOCK g_Lock;
    BOOLEAN g_Initialized = FALSE;

    VOID CopyIntoRing(_In_reads_bytes_(ByteCount) const UCHAR* Source, _In_ ULONG ByteCount, _In_ ULONGLONG Cursor)
    {
        ULONG offset = static_cast<ULONG>(Cursor % g_Capacity);
        ULONG first = min(ByteCount, g_Capacity - offset);
        RtlCopyMemory(g_Buffer + offset, Source, first);
        if (ByteCount > first)
        {
            RtlCopyMemory(g_Buffer, Source + first, ByteCount - first);
        }
    }

    VOID CopyFromRing(_Out_writes_bytes_(ByteCount) UCHAR* Destination, _In_ ULONG ByteCount, _In_ ULONGLONG Cursor)
    {
        ULONG offset = static_cast<ULONG>(Cursor % g_Capacity);
        ULONG first = min(ByteCount, g_Capacity - offset);
        RtlCopyMemory(Destination, g_Buffer + offset, first);
        if (ByteCount > first)
        {
            RtlCopyMemory(Destination + first, g_Buffer, ByteCount - first);
        }
    }
}

NTSTATUS VesperCableInitialize()
{
    if (g_Initialized)
    {
        return STATUS_SUCCESS;
    }

    KeInitializeSpinLock(&g_Lock);
    g_Buffer = static_cast<UCHAR*>(ExAllocatePool2(POOL_FLAG_NON_PAGED, VESPER_CABLE_CAPACITY_BYTES, VESPER_CABLE_POOLTAG));
    if (g_Buffer == nullptr)
    {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    RtlZeroMemory(g_Buffer, VESPER_CABLE_CAPACITY_BYTES);
    g_Capacity = VESPER_CABLE_CAPACITY_BYTES;
    g_ReadCursor = 0;
    g_WriteCursor = 0;
    g_Initialized = TRUE;
    return STATUS_SUCCESS;
}

VOID VesperCableCleanup()
{
    if (!g_Initialized)
    {
        return;
    }

    if (g_Buffer != nullptr)
    {
        ExFreePoolWithTag(g_Buffer, VESPER_CABLE_POOLTAG);
        g_Buffer = nullptr;
    }
    g_Capacity = 0;
    g_ReadCursor = 0;
    g_WriteCursor = 0;
    g_Initialized = FALSE;
}

VOID VesperCablePush(_In_reads_bytes_(ByteCount) const UCHAR* Source, _In_ ULONG ByteCount)
{
    if (!g_Initialized || Source == nullptr || ByteCount == 0)
    {
        return;
    }

    if (ByteCount >= g_Capacity)
    {
        Source += ByteCount - g_Capacity;
        ByteCount = g_Capacity;
    }

    KIRQL oldIrql;
    KeAcquireSpinLock(&g_Lock, &oldIrql);
    CopyIntoRing(Source, ByteCount, g_WriteCursor);
    g_WriteCursor += ByteCount;
    if (g_WriteCursor - g_ReadCursor > g_Capacity)
    {
        g_ReadCursor = g_WriteCursor - g_Capacity;
    }
    KeReleaseSpinLock(&g_Lock, oldIrql);
}

VOID VesperCablePop(_Out_writes_bytes_(ByteCount) UCHAR* Destination, _In_ ULONG ByteCount)
{
    if (Destination == nullptr || ByteCount == 0)
    {
        return;
    }

    RtlZeroMemory(Destination, ByteCount);
    if (!g_Initialized)
    {
        return;
    }

    KIRQL oldIrql;
    KeAcquireSpinLock(&g_Lock, &oldIrql);
    ULONG available = static_cast<ULONG>(min(g_WriteCursor - g_ReadCursor, static_cast<ULONGLONG>(g_Capacity)));
    ULONG copyCount = min(ByteCount, available);
    if (copyCount > 0)
    {
        CopyFromRing(Destination, copyCount, g_ReadCursor);
        g_ReadCursor += copyCount;
    }
    KeReleaseSpinLock(&g_Lock, oldIrql);
}
