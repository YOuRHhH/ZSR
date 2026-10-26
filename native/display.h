#ifndef ZSR_DISPLAY_H
#define ZSR_DISPLAY_H
#include <stdint.h>
/* Caller owns the buffer. Writes at most capacity items; returns count or -1.
 * Sizes are capture pixels. Origins are physical desktop pixels on Windows/X11
 * and CoreGraphics desktop points on macOS.
 * No allocation or retained pointers cross this boundary. Call on worker threads.
 */
typedef struct { int32_t x, y, width, height; uint32_t id; } ZsrDisplay;
int32_t zsr_displays(ZsrDisplay *out, int32_t capacity);
#endif
