#include "display.h"
#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>

/* Resolve the selected display by its full rectangle, never by an unrelated API's index. */
uint64_t zsr_display_monitor(const ZsrDisplay *display)
{
    if (!display || display->width < 2 || display->height < 2)
        return 0;
    RECT rect = {display->x, display->y,
                 display->x + display->width, display->y + display->height};
    HMONITOR monitor = MonitorFromRect(&rect, MONITOR_DEFAULTTONULL);
    MONITORINFO info = {0};
    info.cbSize = sizeof(info);
    if (!monitor || !GetMonitorInfoW(monitor, &info) || !EqualRect(&rect, &info.rcMonitor))
        return 0;
    return (uint64_t)(uintptr_t)monitor;
}
#endif
