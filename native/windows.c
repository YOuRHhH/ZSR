#include <stdint.h>
#if defined(_WIN32)
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <dwmapi.h>
typedef struct { uint64_t id; uint32_t pid; uint16_t title[512], app[260]; } ZsrWindow;
typedef struct { ZsrWindow *out; int capacity, count; } WindowList;
static BOOL CALLBACK collect_window(HWND window, LPARAM data) {
    WindowList *list = (WindowList *)data;
    DWORD pid = 0, cloaked = 0;
    RECT rect;
    GetWindowThreadProcessId(window, &pid);
    if (pid == GetCurrentProcessId() || !IsWindowVisible(window) || IsIconic(window)
        || (GetWindowLongPtrW(window, GWL_EXSTYLE) & WS_EX_TOOLWINDOW)
        || !GetClientRect(window, &rect) || rect.right < 2 || rect.bottom < 2) return TRUE;
    if (SUCCEEDED(DwmGetWindowAttribute(window, DWMWA_CLOAKED, &cloaked, sizeof(cloaked))) && cloaked) return TRUE;
    ZsrWindow item = {0};
    if (!GetWindowTextW(window, (wchar_t *)item.title, 512)) return TRUE;
    item.id = (uint64_t)(uintptr_t)window; item.pid = pid;
    HANDLE process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
    if (process) {
        DWORD length = 260;
        QueryFullProcessImageNameW(process, 0, (wchar_t *)item.app, &length);
        CloseHandle(process);
    }
    if (list->count < list->capacity) list->out[list->count++] = item;
    return TRUE;
}
int32_t zsr_windows(ZsrWindow *out, int32_t capacity) {
    if (!out || capacity < 1) return -1;
    WindowList list = {out, capacity, 0};
    return EnumWindows(collect_window, (LPARAM)&list) ? list.count : -1;
}
#endif
