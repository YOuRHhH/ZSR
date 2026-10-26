#include "display.h"
#include <stdio.h>
#include <stdarg.h>
/* Keep libobs diagnostics off the helper's JSON protocol stdout. */
static void log_obs(int level, const char *format, va_list args, void *context)
{
    (void)level; (void)context;
    vfprintf(stderr, format, args);
    fputc('\n', stderr);
}
/* Expose the C variadic callback without reproducing va_list ABI in Rust. */
void *zsr_obs_log_handler(void) { return (void *)log_obs; }
#ifdef _WIN32
#include <windows.h>
extern uint64_t zsr_display_monitor(const ZsrDisplay *display);
/* Match OBS's stable monitor_id using the actual selected monitor, not enumeration order. */
int zsr_obs_monitor_id(const ZsrDisplay *display, char *out, int capacity)
{
    HMONITOR monitor = (HMONITOR)(uintptr_t)zsr_display_monitor(display);
    MONITORINFOEXW info = {0}; info.cbSize = sizeof(info);
    DISPLAY_DEVICEW device = {0}; device.cb = sizeof(device);
    if (!monitor || !GetMonitorInfoW(monitor, (MONITORINFO *)&info)) return 0;
    const wchar_t *id = info.szDevice;
    if (EnumDisplayDevicesW(info.szDevice, 0, &device, EDD_GET_DEVICE_INTERFACE_NAME)) id = device.DeviceID;
    return WideCharToMultiByte(CP_UTF8, 0, id, -1, out, capacity, NULL, NULL);
}
#endif
