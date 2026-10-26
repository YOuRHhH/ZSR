#include <stdint.h>
#if defined(_WIN32)
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdlib.h>
typedef struct { HWND edges[4]; } ZsrMarker;

static LRESULT CALLBACK marker_proc(HWND window, UINT message, WPARAM w, LPARAM l) {
    if (message == WM_NCHITTEST) return HTTRANSPARENT;
    if (message == WM_MOUSEACTIVATE) return MA_NOACTIVATE;
    if (message == WM_PAINT) {
        PAINTSTRUCT ps;
        HDC dc = BeginPaint(window, &ps);
        HBRUSH brush = CreateSolidBrush(RGB(90,185,255));
        FillRect(dc, &ps.rcPaint, brush);
        DeleteObject(brush);
        EndPaint(window, &ps);
        return 0;
    }
    return DefWindowProcW(window,message,w,l);
}

void zsr_marker_destroy(ZsrMarker *marker) {
    if (!marker) return;
    for (int i=0;i<4;++i) if (marker->edges[i]) DestroyWindow(marker->edges[i]);
    free(marker);
}

ZsrMarker *zsr_marker_create(void) {
    WNDCLASSW cls = {0};
    cls.lpfnWndProc = marker_proc;
    cls.hInstance = GetModuleHandleW(NULL);
    cls.lpszClassName = L"ZSR.RegionMarker";
    if (!RegisterClassW(&cls) && GetLastError() != ERROR_CLASS_ALREADY_EXISTS) return NULL;
    ZsrMarker *marker = (ZsrMarker *)calloc(1,sizeof(ZsrMarker));
    if (!marker) return NULL;
    for (int i=0;i<4;++i) {
        marker->edges[i] = CreateWindowExW(WS_EX_LAYERED | WS_EX_TRANSPARENT |
            WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            cls.lpszClassName,L"ZSR recording area",WS_POPUP,0,0,0,0,
            NULL,NULL,cls.hInstance,NULL);
        if (!marker->edges[i] || !SetLayeredWindowAttributes(marker->edges[i],0,255,LWA_ALPHA)) {
            zsr_marker_destroy(marker); return NULL;
        }
        /* Best effort on supported Windows versions. Edges also sit outside
         * the recording rectangle so they do not contaminate region capture. */
        SetWindowDisplayAffinity(marker->edges[i],0x11);
    }
    return marker;
}

int32_t zsr_marker_position(ZsrMarker *marker, int32_t x, int32_t y, int32_t width, int32_t height) {
    if (!marker || width<2 || height<2) return 0;
    DPI_AWARENESS_CONTEXT old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    const int t = 2;
    int edges[4][4] = {{x-t,y-t,width+2*t,t},{x-t,y+height,width+2*t,t},
                       {x-t,y,t,height},{x+width,y,t,height}};
    int ok = 1;
    for (int i=0;i<4;++i) {
        if (!SetWindowPos(marker->edges[i],HWND_TOPMOST,edges[i][0],edges[i][1],edges[i][2],edges[i][3],
            SWP_NOACTIVATE | SWP_SHOWWINDOW)) ok = 0;
    }
    if (old) SetThreadDpiAwarenessContext(old);
    return ok;
}
/* Off-desktop native regression check; does not obscure user applications. */
int32_t zsr_marker_verify(void) {
    ZsrMarker *marker = zsr_marker_create();
    if (!marker) return 1;
    int x = GetSystemMetrics(SM_XVIRTUALSCREEN)-400;
    int y = GetSystemMetrics(SM_YVIRTUALSCREEN)-400;
    int error = zsr_marker_position(marker,x,y,100,80) ? 0 : 2;
    RECT capture = {x,y,x+100,y+80};
    RECT expected[4] = {{x-2,y-2,x+102,y},{x-2,y+80,x+102,y+82},
                        {x-2,y,x,y+80},{x+100,y,x+102,y+80}};
    HWND handles[4];
    for (int i=0;i<4;++i) {
        handles[i] = marker->edges[i];
        RECT actual = {0}, overlap;
        if (!GetWindowRect(handles[i],&actual) || !EqualRect(&actual,&expected[i])) error |= 4;
        if (IntersectRect(&overlap,&actual,&capture)) error |= 8;
        LONG_PTR style = GetWindowLongPtrW(handles[i],GWL_EXSTYLE);
        LONG_PTR required = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOPMOST;
        if ((style & required) != required) error |= 16;
    }
    zsr_marker_destroy(marker);
    for (int i=0;i<4;++i) if (IsWindow(handles[i])) error |= 32;
    return error;
}
#endif
