#include <stdint.h>
#if defined(_WIN32)
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <windowsx.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

typedef struct { int32_t kind, x, y; } BrushEvent;
/* Paths and thickness arrive in source pixels; color uses Win32 COLORREF byte order. */
typedef struct { uint32_t offset, count; float width; uint32_t color; } BrushStroke;
typedef struct {
    HWND window, previous_focus, tools;
    HDC dc;
    HBITMAP bitmap;
    HGDIOBJ previous_bitmap;
    uint32_t *pixels;
    int width, height, enabled, drawing, count;
    BrushEvent events[256];
} ZsrBrush;

/* Keep the independent tools clickable when drawing activates/raises the full-screen canvas.
   Move only our canvas, on its owning thread, and only when the relative order is wrong. */
static void keep_tools_above(ZsrBrush *b) {
    if (!IsWindow(b->tools) || !IsWindowVisible(b->tools)) return;
    for (HWND above = GetWindow(b->tools, GW_HWNDPREV); above; above = GetWindow(above, GW_HWNDPREV)) {
        if (above == b->window) {
            SetWindowPos(b->window, b->tools, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            break;
        }
    }
}
void zsr_brush_tools(ZsrBrush *b, const wchar_t *title) {
    HWND tools = FindWindowW(NULL, title);
    DWORD pid = 0;
    GetWindowThreadProcessId(tools, &pid);
    b->tools = pid == GetCurrentProcessId() ? tools : NULL;
    keep_tools_above(b);
}

/* Input and presentation belong to this worker's native window, not the egui GL loop. */
static void enqueue(ZsrBrush *b, int kind, LPARAM l) {
    if (b->count == 256) {
        /* Discard a move, never a button release or Escape, when a burst fills the queue. */
        for (int i = 0; i < b->count; ++i) if (b->events[i].kind == 2) {
            memmove(b->events+i, b->events+i+1, (size_t)(--b->count-i)*sizeof(BrushEvent));
            break;
        }
    }
    if (b->count < 256) b->events[b->count++] = (BrushEvent){kind, GET_X_LPARAM(l), GET_Y_LPARAM(l)};
}

void zsr_brush_mode(ZsrBrush *b, int enabled) {
    if (!b || b->enabled == enabled) return;
    b->enabled = enabled;
    LONG_PTR style = GetWindowLongPtrW(b->window, GWL_EXSTYLE);
    if (enabled) style &= ~(WS_EX_TRANSPARENT | WS_EX_NOACTIVATE);
    else style |= WS_EX_TRANSPARENT | WS_EX_NOACTIVATE;
    SetWindowLongPtrW(b->window, GWL_EXSTYLE, style);
    SetWindowPos(b->window, HWND_TOPMOST, 0, 0, 0, 0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED);
    if (enabled) {
        b->previous_focus = GetForegroundWindow();
        /* Escape remains an exit path even if another application retains keyboard focus. */
        RegisterHotKey(b->window, 1, MOD_NOREPEAT, VK_ESCAPE);
        SetForegroundWindow(b->window);
        SetFocus(b->window);
    } else {
        UnregisterHotKey(b->window, 1);
        b->drawing = 0;
        if (GetCapture() == b->window) ReleaseCapture();
        HWND foreground = GetForegroundWindow();
        if ((foreground == b->window || foreground == b->tools) && IsWindow(b->previous_focus))
            SetForegroundWindow(b->previous_focus);
    }
}

static LRESULT CALLBACK brush_proc(HWND window, UINT message, WPARAM w, LPARAM l) {
    ZsrBrush *b = (ZsrBrush *)GetWindowLongPtrW(window, GWLP_USERDATA);
    if (message == WM_NCCREATE) {
        b = (ZsrBrush *)((CREATESTRUCTW *)l)->lpCreateParams;
        b->window = window;
        SetWindowLongPtrW(window, GWLP_USERDATA, (LONG_PTR)b);
    }
    if (!b) return DefWindowProcW(window, message, w, l);
    switch (message) {
    case WM_WINDOWPOSCHANGED:
        keep_tools_above(b);
        return DefWindowProcW(window, message, w, l);
    case WM_ERASEBKGND: return 1;
    case WM_PAINT: { PAINTSTRUCT paint; BeginPaint(window, &paint); EndPaint(window, &paint); return 0; }
    case WM_LBUTTONDOWN:
        if (b->enabled) { b->drawing = 1; SetCapture(window); enqueue(b, 1, l); } return 0;
    case WM_MOUSEMOVE: if (b->enabled && b->drawing) enqueue(b, 2, l); return 0;
    case WM_LBUTTONUP:
        if (b->enabled && b->drawing) { enqueue(b, 3, l); b->drawing = 0; ReleaseCapture(); } return 0;
    case WM_CAPTURECHANGED: b->drawing = 0; enqueue(b, 7, 0); return 0;
    case WM_KEYDOWN:
        if (b->enabled && w == VK_ESCAPE) { zsr_brush_mode(b, 0); enqueue(b, 4, 0); }
        if (b->enabled && w == 'Z' && (GetKeyState(VK_CONTROL) & 0x8000)) enqueue(b, 5, 0);
        if (b->enabled && w == VK_DELETE) enqueue(b, 6, 0);
        return 0;
    case WM_HOTKEY:
        if (w == 1 && b->enabled) { zsr_brush_mode(b, 0); enqueue(b, 4, 0); } return 0;
    case WM_MOUSEACTIVATE: return b->enabled ? MA_ACTIVATE : MA_NOACTIVATE;
    case WM_CLOSE: case WM_DISPLAYCHANGE:
        zsr_brush_mode(b, 0); enqueue(b, 4, 0); return 0;
    default: return DefWindowProcW(window, message, w, l);
    }
}

ZsrBrush *zsr_brush_create(void) {
    /* This thread exits with the overlay, so its DPI context needs no global process change. */
    SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    WNDCLASSW cls = {0};
    cls.lpfnWndProc = brush_proc;
    cls.hInstance = GetModuleHandleW(NULL);
    cls.hCursor = LoadCursorW(NULL, IDC_CROSS);
    cls.lpszClassName = L"ZSR.NativeBrush";
    if (!RegisterClassW(&cls) && GetLastError() != ERROR_CLASS_ALREADY_EXISTS) return NULL;
    ZsrBrush *b = (ZsrBrush *)calloc(1, sizeof(*b));
    if (!b) return NULL;
    b->window = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST |
        WS_EX_TRANSPARENT | WS_EX_NOACTIVATE, cls.lpszClassName, L"ZSR Drawing Brush",
        WS_POPUP, 0, 0, 1, 1, NULL, NULL, cls.hInstance, b);
    if (!b->window) { free(b); return NULL; }
    return b;
}

static void free_buffer(ZsrBrush *b) {
    if (b->previous_bitmap) SelectObject(b->dc, b->previous_bitmap);
    if (b->bitmap) DeleteObject(b->bitmap);
    if (b->dc) DeleteDC(b->dc);
    b->dc = NULL; b->bitmap = NULL; b->previous_bitmap = NULL; b->pixels = NULL;
}

/* Supply explicit premultiplied BGRA to the compositor, independent of GPU alpha support. */
int zsr_brush_present(ZsrBrush *b, const int32_t *rect, int enabled,
                      const BrushStroke *strokes, uint32_t count, const float *points) {
    int width = rect[2], height = rect[3];
    if (!b || width < 1 || height < 1 || (uint64_t)width * height > 64*1024*1024) return 0;
    if (!b->pixels || width != b->width || height != b->height) {
        free_buffer(b);
        BITMAPINFO info = {0};
        info.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
        info.bmiHeader.biWidth = width; info.bmiHeader.biHeight = -height;
        info.bmiHeader.biPlanes = 1; info.bmiHeader.biBitCount = 32;
        b->dc = CreateCompatibleDC(NULL);
        if (!b->dc) return 0;
        b->bitmap = CreateDIBSection(b->dc, &info, DIB_RGB_COLORS, (void **)&b->pixels, NULL, 0);
        if (!b->bitmap) return 0;
        b->previous_bitmap = SelectObject(b->dc, b->bitmap);
        b->width = width; b->height = height;
    }
    /* GDI clears the alpha byte of painted pixels. Mark untouched pixels with alpha=1
       so opaque black ink remains distinguishable from the transparent background. */
    for (size_t i = 0; i < (size_t)width * height; ++i) b->pixels[i] = 0x01000000;
    for (uint32_t i = 0; i < count; ++i) {
        int thickness = max(1, (int)lroundf(strokes[i].width));
        HBRUSH brush = CreateSolidBrush(strokes[i].color);
        HPEN pen = CreatePen(PS_SOLID, thickness, strokes[i].color);
        if (!brush || !pen) { if (brush) DeleteObject(brush); if (pen) DeleteObject(pen); return 0; }
        HGDIOBJ old_brush = SelectObject(b->dc, brush);
        HGDIOBJ old_pen = SelectObject(b->dc, pen);
        for (uint32_t j = 0; j < strokes[i].count; ++j) {
            const float *p = points + 2 * (strokes[i].offset + j);
            int x = (int)lroundf(p[0]), y = (int)lroundf(p[1]);
            if (j == 0) MoveToEx(b->dc, x, y, NULL); else LineTo(b->dc, x, y);
            /* Explicit dots preserve click-only marks, using a null pen to avoid double width. */
            SelectObject(b->dc, GetStockObject(NULL_PEN));
            Ellipse(b->dc, x-thickness/2, y-thickness/2, x+(thickness+1)/2, y+(thickness+1)/2);
            SelectObject(b->dc, pen);
        }
        SelectObject(b->dc, old_pen); DeleteObject(pen);
        SelectObject(b->dc, old_brush); DeleteObject(brush);
    }
    GdiFlush();
    /* Alpha=1 accepts drawing input without an opaque backdrop; pointer mode is fully clear. */
    for (size_t i = 0; i < (size_t)width * height; ++i)
        b->pixels[i] = b->pixels[i] == 0x01000000 ? (enabled ? 0x01000000 : 0) : b->pixels[i] | 0xff000000;
    POINT origin = {rect[0], rect[1]}, source = {0, 0};
    SIZE size = {width, height};
    BLENDFUNCTION blend = {AC_SRC_OVER, 0, 255, AC_SRC_ALPHA};
    if (!UpdateLayeredWindow(b->window, NULL, &origin, &size, b->dc, &source, 0, &blend, ULW_ALPHA)) return 0;
    ShowWindow(b->window, SW_SHOWNOACTIVATE);
    return 1;
}

int zsr_brush_poll(ZsrBrush *b, BrushEvent *events) {
    MSG message;
    while (PeekMessageW(&message, NULL, 0, 0, PM_REMOVE)) {
        TranslateMessage(&message); DispatchMessageW(&message);
    }
    int count = b->count;
    memcpy(events, b->events, (size_t)count * sizeof(BrushEvent)); b->count = 0;
    return count;
}
void zsr_brush_destroy(ZsrBrush *b) {
    if (!b) return;
    zsr_brush_mode(b, 0);
    DestroyWindow(b->window); free_buffer(b); free(b);
}

/* Regression test uses only its own off-screen window and inspects the actual DIB/styles. */
int zsr_brush_test(void) {
    ZsrBrush *b = zsr_brush_create();
    if (!b) return 1;
    int32_t rect[4] = {-20000, -20000, 80, 60};
    float points[4] = {20.0f, 30.0f, 60.0f, 30.0f};
    BrushStroke stroke = {0, 2, 6.0f, RGB(255, 70, 60)};
    int error = 0;
    if (!zsr_brush_present(b, rect, 1, &stroke, 1, points)) error |= 2;
    else {
        if (b->pixels[0] != 0x01000000 || b->pixels[30*80+40] != 0xffff463c) error |= 4;
        zsr_brush_mode(b, 1);
        if (!b->enabled || (GetWindowLongPtrW(b->window, GWL_EXSTYLE) & WS_EX_TRANSPARENT)) error |= 64;
        SendMessageW(b->window, WM_KEYDOWN, VK_ESCAPE, 0);
        if (b->enabled || !(GetWindowLongPtrW(b->window, GWL_EXSTYLE) & WS_EX_TRANSPARENT)) error |= 8;
        if (!zsr_brush_present(b, rect, 0, &stroke, 1, points) || b->pixels[0] != 0) error |= 16;
        /* The second toggle and the global Escape event must release capture as well. */
        zsr_brush_mode(b, 1); zsr_brush_mode(b, 0);
        if (b->enabled || !(GetWindowLongPtrW(b->window, GWL_EXSTYLE) & WS_EX_TRANSPARENT)) error |= 128;
        zsr_brush_mode(b, 1); SendMessageW(b->window, WM_HOTKEY, 1, 0);
        if (b->enabled) error |= 256;
        /* Black and custom colors must remain opaque in both drawing and pointer modes. */
        stroke.color = RGB(0, 0, 0);
        if (!zsr_brush_present(b, rect, 1, &stroke, 1, points) || b->pixels[30*80+40] != 0xff000000 || b->pixels[0] != 0x01000000) error |= 1024;
        stroke.color = RGB(30, 220, 90);
        if (!zsr_brush_present(b, rect, 0, &stroke, 1, points) || b->pixels[30*80+40] != 0xff1edc5a || b->pixels[0] != 0) error |= 2048;
        if (!zsr_brush_present(b, rect, 0, NULL, 0, NULL) || b->pixels[30*80+40] != 0) error |= 512;
        /* A tools window stays above the canvas even after the drawing window is raised. */
        HWND tools = CreateWindowExW(WS_EX_TOPMOST | WS_EX_TOOLWINDOW, L"STATIC", L"ZSR Brush Panel Test",
            WS_POPUP | WS_VISIBLE, -20000, -20000, 80, 60, NULL, NULL, GetModuleHandleW(NULL), NULL);
        if (!tools) error |= 4096;
        else {
            zsr_brush_tools(b, L"ZSR Brush Panel Test");
            SetWindowPos(b->window, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            for (HWND above = GetWindow(tools, GW_HWNDPREV); above; above = GetWindow(above, GW_HWNDPREV))
                if (above == b->window) error |= 4096;
            DestroyWindow(tools);
        }
    }
    HWND window = b->window;
    zsr_brush_destroy(b);
    if (IsWindow(window)) error |= 32;
    return error;
}
#endif
