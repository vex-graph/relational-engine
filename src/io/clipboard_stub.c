#include "io/clipboard.h"
#include <string.h>

#include "annotation/definition.h"
#include "annotation/overview.h"

#if !defined(__APPLE__)

;;DEFINITION
/**
 * ============================================================================
 * DEFINITION: Clipboard (platform-exclusive stub)
 * ============================================================================
 * Non-Apple fallback for the system clipboard bridge: a procedural Clipboard
 * API backed by a single static 4096-byte char buffer (s_stubText). Text
 * copy/paste round-trips in-process through that buffer with truncation at
 * the 4095-byte bound; image operations always return false because there is
 * no OS pasteboard to exchange pixels with. Zero allocation, zero threads,
 * zero steady-state cost — the stub exists so R4/R5 callers compile and run
 * on Linux/Windows while the macOS NSPasteboard implementation carries the
 * real bridge. The whole file is compiled out under __APPLE__.
 * ============================================================================
 */

;;OVERVIEW
/**
 * ============================================================================
 * MODULE: Clipboard_stub (io/clipboard_stub.c)
 * ============================================================================
 * Platform-exclusive stub for the system clipboard bridge on non-Apple
 * systems without NSPasteboard. Procedural Clipboard API backed by a static
 * char buffer; image operations are unsupported and always return false.
 *
 * STRUCT FIELDS: none — procedural (static char s_stubText[4096] backing store)
 *
 * FUNCTION REGISTRY:
 * ----------------------------------------------------------------------------
 * Getters:
 *   - Clipboard_hasText(void)                    : true when text is stored
 *   - Clipboard_hasImage(void)                   : always false (no pasteboard)
 *   - Clipboard_getText(dest, maxBytes)          : copy stored text, bytes written
 *   - Clipboard_getImage(dest, maxBytes, outWidth, outHeight) : always false
 *
 * Setters:
 *   - Clipboard_setText(text)                    : store text (truncated at 4095)
 *   - Clipboard_setImage(pixels, width, height)  : always false
 *   - Clipboard_clear(void)                      : empty the text store
 * ============================================================================
 */

static char s_stubText[4096] = {0};

bool Clipboard_hasText(void) {
    return s_stubText[0] != '\0';
}

bool Clipboard_hasImage(void) {
    return false;
}

size_t Clipboard_getText(char *dest, size_t maxBytes) {
    if (dest == nullptr || maxBytes == 0) return 0;
    size_t len = strlen(s_stubText);
    if (len >= maxBytes) len = maxBytes - 1;
    memcpy(dest, s_stubText, len);
    dest[len] = '\0';
    return len;
}

bool Clipboard_setText(const char *text) {
    if (text == nullptr) return false;
    size_t len = strlen(text);
    if (len >= sizeof(s_stubText)) len = sizeof(s_stubText) - 1;
    memcpy(s_stubText, text, len);
    s_stubText[len] = '\0';
    return true;
}

bool Clipboard_getImage(void *dest, size_t maxBytes, int32_t *outWidth, int32_t *outHeight) {
    (void) dest; (void) maxBytes; (void) outWidth; (void) outHeight;
    return false;
}

bool Clipboard_setImage(const void *pixels, int32_t width, int32_t height) {
    (void) pixels; (void) width; (void) height;
    return false;
}

void Clipboard_clear(void) {
    s_stubText[0] = '\0';
}

#endif
