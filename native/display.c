#include "display.h"
#if defined(_WIN32)
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
typedef struct { ZsrDisplay *out; int32_t capacity, count; } Context;
static BOOL CALLBACK collect(HMONITOR monitor, HDC dc, LPRECT rect, LPARAM data) {
    Context *ctx = (Context *)data;
    (void)monitor; (void)dc;
    if (ctx->count < ctx->capacity) {
        ZsrDisplay *d = &ctx->out[ctx->count];
        d->x = rect->left; d->y = rect->top;
        d->width = rect->right - rect->left; d->height = rect->bottom - rect->top;
        d->id = (uint32_t)ctx->count;
        ctx->count++;
    }
    return TRUE;
}
int32_t zsr_displays(ZsrDisplay *out, int32_t capacity) {
    Context ctx = {out, capacity, 0};
    if (!out || capacity <= 0) return -1;
    return EnumDisplayMonitors(NULL, NULL, collect, (LPARAM)&ctx) ? ctx.count : -1;
}
#elif defined(__APPLE__)
#include <CoreGraphics/CoreGraphics.h>
int32_t zsr_displays(ZsrDisplay *out, int32_t capacity) {
    CGDirectDisplayID ids[32]; uint32_t count = 0;
    if (!out || capacity <= 0) return -1;
    if (CGGetActiveDisplayList(32, ids, &count) != kCGErrorSuccess) return -1;
    if (count > (uint32_t)capacity) count = (uint32_t)capacity;
    for (uint32_t i = 0; i < count; i++) {
        CGRect bounds = CGDisplayBounds(ids[i]);
        out[i] = (ZsrDisplay){(int32_t)bounds.origin.x, (int32_t)bounds.origin.y,
            (int32_t)CGDisplayPixelsWide(ids[i]), (int32_t)CGDisplayPixelsHigh(ids[i]), ids[i]};
    }
    return (int32_t)count;
}
#elif defined(__linux__)
#include <dlfcn.h>
/* Runtime loading avoids requiring X11 development headers for the adapter.
 * Initial Linux support exposes the complete default X11 screen as one source.
 */
int32_t zsr_displays(ZsrDisplay *out, int32_t capacity) {
    if (!out || capacity <= 0) return -1;
    void *lib = dlopen("libX11.so.6", RTLD_NOW | RTLD_LOCAL);
    if (!lib) return -1;
    void *(*open_display)(const char *) = (void *(*)(const char *))dlsym(lib, "XOpenDisplay");
    int (*default_screen)(void *) = (int (*)(void *))dlsym(lib, "XDefaultScreen");
    int (*width)(void *, int) = (int (*)(void *, int))dlsym(lib, "XDisplayWidth");
    int (*height)(void *, int) = (int (*)(void *, int))dlsym(lib, "XDisplayHeight");
    int (*close_display)(void *) = (int (*)(void *))dlsym(lib, "XCloseDisplay");
    if (!open_display || !default_screen || !width || !height || !close_display) { dlclose(lib); return -1; }
    void *display = open_display(0);
    if (!display) { dlclose(lib); return -1; }
    int screen = default_screen(display);
    out[0] = (ZsrDisplay){0, 0, width(display, screen), height(display, screen), 0};
    close_display(display); dlclose(lib); return 1;
}
#else
int32_t zsr_displays(ZsrDisplay *out, int32_t capacity) { (void)out; (void)capacity; return -1; }
#endif
