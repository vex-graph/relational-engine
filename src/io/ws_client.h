#ifndef IO_WS_CLIENT_H
#define IO_WS_CLIENT_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "c23/constructor.h"

// io/ws_client.h — engine-owned R2 bounded frame slot, driven by R1.
//
// A bounded, thread-free WebSocket frame slot: fixed rx buffer, connection
// state, cancel flag, and timeout. Owns NO sockets and spawns NO threads —
// Bytes arrive via WsClient_feed from the R1 driver (which owns the socket),
// and WsClient_poll drains pending Bytes under the Bounded Wait Law.

#define WS_CLIENT_RX_CAP 4096u
#define WS_CLIENT_POLL_MAX_NS 100000000ULL

typedef enum WsClientState {
    WS_CLIENT_IDLE = 0,
    WS_CLIENT_CONNECTING = 1,
    WS_CLIENT_OPEN = 2,
    WS_CLIENT_CLOSING = 3,
    WS_CLIENT_CLOSED = 4
} WsClientState;

typedef struct WsClient {
    WsClientState state;              // connection state machine step
    uint8_t rxBuf[WS_CLIENT_RX_CAP];  // fixed pending-byte buffer (no alloc)
    uint32_t rxLen;                   // pending Bytes in rxBuf (0..RX_CAP)
    uint64_t timeoutNs;               // default budget capped by the Bounded Wait Law
    bool cancelled;                   // cancel flag; poll drops on sight
} WsClient;

// Idle client, zero budget override (timeoutNs 0 = poll arg governs).
WsClient *WsClient_0(void);

// Client with a default poll budget in nanoseconds (clamped to MAX_NS).
WsClient *WsClient_1(uint64_t timeoutNs);

#define WsClient(...) CONSTRUCTOR_DISPATCH(WsClient, __VA_ARGS__)

// Release the client block (null-safe no-op).
void WsClient_free(WsClient *self);

// Append caller Bytes into the fixed rx buffer. False on NULL self/Bytes,
// zero len, or overflow (buffer keeps old Bytes, caller retries after poll).
bool WsClient_feed(WsClient *self, const uint8_t *Bytes, uint32_t len);

// Drain pending Bytes into dest (Semantic Consistency Law, Argument order). Waits at
// most timeoutNs (clamped to WS_CLIENT_POLL_MAX_NS) in ~1ms slices,
// re-checking the cancel flag each slice; returns false (drop-degrade,
// *outLen 0) on timeout, cancel, empty buffer, or NULL args.
bool WsClient_poll(WsClient *self, uint64_t timeoutNs, uint8_t *dest,
                   uint32_t destCap, uint32_t *outLen);

// Raise the cancel flag (in-flight poll drops at the next slice).
void WsClient_cancel(WsClient *self);

// Discard pending Bytes without delivering (core reset, keeps state).
void WsClient_clear(WsClient *self);

// Symmetric accessors (Single Class Per File Law, symmetric-accessors clause).
void WsClient_setState(WsClient *self, WsClientState state);
WsClientState WsClient_getState(const WsClient *self);
void WsClient_setTimeoutNs(WsClient *self, uint64_t timeoutNs);
uint64_t WsClient_getTimeoutNs(const WsClient *self);
void WsClient_setCancelled(WsClient *self, bool cancelled);
bool WsClient_isCancelled(const WsClient *self);
uint32_t WsClient_getPending(const WsClient *self);

#endif
