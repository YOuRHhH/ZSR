#include "picker.h"
#if defined(_WIN32)
#include <windowsx.h>

static LRESULT CALLBACK picker_proc(HWND window, UINT message, WPARAM w, LPARAM l) {
    ZsrPicker *p = (ZsrPicker *)GetWindowLongPtrW(window, GWLP_USERDATA);
    if (message == WM_NCCREATE) {
        p = (ZsrPicker *)((CREATESTRUCTW *)l)->lpCreateParams;
        SetWindowLongPtrW(window, GWLP_USERDATA, (LONG_PTR)p);
        p->window = window;
    }
    if (!p) return DefWindowProcW(window, message, w, l);
    POINT point = {GET_X_LPARAM(l), GET_Y_LPARAM(l)};
    switch (message) {
    case WM_ERASEBKGND: return 1;
    case WM_LBUTTONDOWN:
        SetCapture(window); zsr_picker_down(p, point); return 0;
    case WM_MOUSEMOVE: zsr_picker_move(p, point); return 0;
    case WM_LBUTTONUP:
        zsr_picker_up(p, point); ReleaseCapture(); return 0;
    case WM_CAPTURECHANGED: p->drag = 0; p->pressed = 0; return 0;
    case WM_KEYDOWN:
        if (w == VK_ESCAPE) p->result = 2;
        if (w == VK_RETURN && !p->drag && p->selection.right-p->selection.left >= 2
            && p->selection.bottom-p->selection.top >= 2) p->result = 1;
        return 0;
    case WM_TIMER:
        if (p->dirty) {
            p->dirty = 0;
            if (!zsr_picker_present(p)) p->result = -1;
        }
        return 0;
    case WM_ACTIVATE:
        if (LOWORD(w) == WA_INACTIVE) p->result = 2;
        return 0;
    case WM_CLOSE: case WM_DISPLAYCHANGE: p->result = 2; return 0;
    default: return DefWindowProcW(window, message, w, l);
    }
}

/* Synchronous on a dedicated Rust worker thread; no egui/GL window is involved.
 * Input is in physical display-relative coordinates under per-monitor DPI v2. */
int32_t zsr_pick_region(const ZsrDisplay *display, int32_t *rect,
                       const wchar_t *instructions, const wchar_t *confirm,
                       const wchar_t *cancel) {
    DPI_AWARENESS_CONTEXT old_dpi = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    WNDCLASSW cls = {0};
    cls.lpfnWndProc = picker_proc;
    cls.hInstance = GetModuleHandleW(NULL);
    cls.hCursor = LoadCursorW(NULL, IDC_CROSS);
    cls.lpszClassName = L"ZSR.NativeRegionPicker";
    if (!RegisterClassW(&cls) && GetLastError() != ERROR_CLASS_ALREADY_EXISTS) {
        if (old_dpi) SetThreadDpiAwarenessContext(old_dpi);
        return -1;
    }
    ZsrPicker p = {0};
    p.display = *display;
    p.instructions = instructions; p.confirm_text = confirm; p.cancel_text = cancel;
    p.selection = (RECT){max(0,min(display->width-2,rect[0])),max(0,min(display->height-2,rect[1])),0,0};
    p.selection.right = p.selection.left + min(display->width-p.selection.left, max(2,rect[2]));
    p.selection.bottom = p.selection.top + min(display->height-p.selection.top, max(2,rect[3]));
    HWND window = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        cls.lpszClassName, L"ZSR Region Picker", WS_POPUP,
        display->x, display->y, display->width, display->height,
        NULL, NULL, cls.hInstance, &p);
    if (!window || !zsr_picker_buffer(&p) || !zsr_picker_present(&p)) p.result = -1;
    if (!p.result) {
        ShowWindow(window, SW_SHOW);
        SetForegroundWindow(window);
        SetFocus(window);
        if (!SetTimer(window, 1, 16, NULL)) p.result = -1;
        MSG message;
        while (!p.result) {
            int status = GetMessageW(&message, NULL, 0, 0);
            if (status <= 0) { p.result = status < 0 ? -1 : 2; break; }
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    int result = p.result;
    if (window) { KillTimer(window, 1); DestroyWindow(window); }
    zsr_picker_free(&p);
    if (old_dpi) SetThreadDpiAwarenessContext(old_dpi);
    if (result == 1) {
        rect[0] = p.selection.left; rect[1] = p.selection.top;
        rect[2] = p.selection.right-p.selection.left; rect[3] = p.selection.bottom-p.selection.top;
    }
    return result;
}
#endif
