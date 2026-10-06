#include "io/ws_client.h"

#include <string.h>
#include <time.h>

#include "nio/mem.h"
#include "oop/type.h"
#include "annotation/definition.h"
#include "annotation/overview.h"

;;DEFINITION
/**
 * ============================================================================
 * DEFINITION: WsClient
 * ============================================================================
 * A bounded, thread-free WebSocket frame slot for the R1 leaf layer. Fixed rx
 * buffer, connection state, cancel flag, and timeout — zero steady-state
 * allocation, no threads, no sockets. Bytes arrive via WsClient_feed from the
 * R0 driver (which owns the socket); WsClient_poll drains pending bytes with a
 * bounded wait: timeoutNs clamped to WS_CLIENT_POLL_MAX_NS (100ms), ~1ms
 * nanosleep slices, cancel flag re-checked each slice, false (drop-degrade)
 * on timeout per the Bounded Wait Law.
 * ============================================================================
 */

;;OVERVIEW
/**
 * ============================================================================
 * CLASS: WsClient (io/ws_client.c)
 * ============================================================================
 * A bounded, thread-free WebSocket frame slot for the R1 leaf layer.
 * Fixed rx buffer, connection state, cancel flag, and timeout — zero
 * steady-state allocation, no threads, no sockets. Bytes arrive via
 * WsClient_feed from the R0 driver (which owns the socket); WsClient_poll
 * drains pending bytes with a bounded wait: timeoutNs clamped to
 * WS_CLIENT_POLL_MAX_NS (100ms, Rule 27), ~1ms nanosleep slices,
 * cancel flag re-checked each slice, false (drop-degrade) on timeout.
 *
 * STRUCT FIELDS (Mirroring io/ws_client.h — exactly this file's class):
 * ----------------------------------------------------------------------------
 *   WsClient {
 *     WsClientState state;             // IDLE/CONNECTING/OPEN/CLOSING/CLOSED
 *     uint8_t rxBuf[WS_CLIENT_RX_CAP]; // fixed 4096B pending buffer, no alloc
 *     uint32_t rxLen;                  // pending bytes (0..RX_CAP)
 *     uint64_t timeoutNs;              // default poll budget, clamped Rule 27
 *     bool cancelled;                  // cancel flag; poll drops on sight
 *   }
 *
 * FUNCTION REGISTRY:
 * ----------------------------------------------------------------------------
 * Constructors:
 *   - WsClient()               : WsClient_0()
 *   - WsClient(timeoutNs)      : WsClient_1(timeoutNs)
 *
 * Core Functions:
 *   - WsClient_free(self)
 *   - WsClient_feed(self, bytes, len)
 *   - WsClient_poll(self, timeoutNs, dest, destCap, outLen)
 *   - WsClient_cancel(self)
 *   - WsClient_clear(self)
 *
 * Setters:
 *   - WsClient_setState(self, state)
 *   - WsClient_setTimeoutNs(self, timeoutNs)
 *   - WsClient_setCancelled(self, cancelled)
 *
 * Getters:
 *   - WsClient_getState(self)
 *   - WsClient_getTimeoutNs(self)
 *   - WsClient_isCancelled(self)
 *   - WsClient_getPending(self)
 * ============================================================================
 */

// io/ws_client.c — WsClient port. Bounded frame slot, R0-fed, never blocks
// past WS_CLIENT_POLL_MAX_NS and never touches a socket or a thread.

static uint64_t wsClientNowNs(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t) ts.tv_sec * 1000000000ULL + (uint64_t) ts.tv_nsec;
}

static void wsClientSleepSlice(void) {
    struct timespec slice;
    slice.tv_sec = 0;
    slice.tv_nsec = 1000000L;
    nanosleep(&slice, nullptr);
}

static WsClient *wsClientCreate(uint64_t timeoutNs) {
    WsClient *self = (WsClient*) Memory_alloc(TYPE_WS_CLIENT_SINGLETON, sizeof(WsClient));
    if (!self)
        return nullptr;
    (*self).state = WS_CLIENT_IDLE;
    (*self).rxLen = 0;
    if (timeoutNs > WS_CLIENT_POLL_MAX_NS)
        timeoutNs = WS_CLIENT_POLL_MAX_NS;
    (*self).timeoutNs = timeoutNs;
    (*self).cancelled = false;
    return self;
}

// CONSTRUCTORS
WsClient *WsClient_0(void) {
    return wsClientCreate(0);
}

WsClient *WsClient_1(uint64_t timeoutNs) {
    return wsClientCreate(timeoutNs);
}

// CORE FUNCTIONS
void WsClient_free(WsClient *self) {
    if (!self)
        return;
    Memory_free(self);
}

bool WsClient_feed(WsClient *self, const uint8_t *bytes, uint32_t len) {
    if (!self || !bytes)
        return false;
    if (len == 0)
        return false;
    if (len > WS_CLIENT_RX_CAP - (*self).rxLen)
        return false;
    memcpy((*self).rxBuf + (*self).rxLen, bytes, len);
    (*self).rxLen += len;
    return true;
}

bool WsClient_poll(WsClient *self, uint64_t timeoutNs, uint8_t *dest,
                   uint32_t destCap, uint32_t *outLen) {
    if (outLen)
        (*outLen) = 0;
    if (!self || !dest || destCap == 0 || !outLen)
        return false;
    if (timeoutNs > WS_CLIENT_POLL_MAX_NS)
        timeoutNs = WS_CLIENT_POLL_MAX_NS;
    if ((*self).cancelled)
        return false;
    if ((*self).rxLen == 0 && timeoutNs == 0)
        return false;
    uint64_t start = wsClientNowNs();
    for (;;) {
        if ((*self).cancelled)
            return false;
        if ((*self).rxLen > 0)
            break;
        if (wsClientNowNs() - start >= timeoutNs)
            return false;
        wsClientSleepSlice();
    }
    uint32_t n = (*self).rxLen;
    if (n > destCap)
        return false;
    memcpy(dest, (*self).rxBuf, n);
    (*self).rxLen = 0;
    (*outLen) = n;
    return true;
}

void WsClient_cancel(WsClient *self) {
    if (!self)
        return;
    (*self).cancelled = true;
}

void WsClient_clear(WsClient *self) {
    if (!self)
        return;
    (*self).rxLen = 0;
}

// SETTERS
void WsClient_setState(WsClient *self, WsClientState state) {
    if (!self)
        return;
    (*self).state = state;
}

void WsClient_setTimeoutNs(WsClient *self, uint64_t timeoutNs) {
    if (!self)
        return;
    if (timeoutNs > WS_CLIENT_POLL_MAX_NS)
        timeoutNs = WS_CLIENT_POLL_MAX_NS;
    (*self).timeoutNs = timeoutNs;
}

void WsClient_setCancelled(WsClient *self, bool cancelled) {
    if (!self)
        return;
    (*self).cancelled = cancelled;
}

// GETTERS
WsClientState WsClient_getState(const WsClient *self) {
    if (!self)
        return WS_CLIENT_CLOSED;
    return (*self).state;
}

uint64_t WsClient_getTimeoutNs(const WsClient *self) {
    if (!self)
        return 0;
    return (*self).timeoutNs;
}

bool WsClient_isCancelled(const WsClient *self) {
    if (!self)
        return true;
    return (*self).cancelled;
}

uint32_t WsClient_getPending(const WsClient *self) {
    if (!self)
        return 0;
    return (*self).rxLen;
}
