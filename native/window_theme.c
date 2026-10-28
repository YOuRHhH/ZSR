#ifdef _WIN32
#include <windows.h>
#include <dwmapi.h>
/* Reveal a centered portion of a fixed-size renderer. Only the clipping region
 * changes during startup: the HWND, swapchain and UI layout retain their final
 * dimensions. A zero radius restores the normal application frame. */
int zsr_set_splash_frame(void *window, int width, int height, int radius)
{
    HWND hwnd = (HWND)window;
    if (!hwnd) return 0;
    if (radius <= 0) return SetWindowRgn(hwnd, NULL, TRUE) != 0;
    RECT rect;
    if (!GetWindowRect(hwnd, &rect)) return 0;
    int left = ((rect.right - rect.left) - width) / 2;
    int top = ((rect.bottom - rect.top) - height) / 2;
    HRGN region = CreateRoundRectRgn(left, top, left + width + 1,
                                   top + height + 1, radius * 2, radius * 2);
    if (!region) return 0;
    /* Windows owns the region only after a successful SetWindowRgn call. */
    if (!SetWindowRgn(hwnd, region, TRUE)) {
        DeleteObject(region);
        return 0;
    }
    return 1;
}

/* Match this window's native frame to ZSR without changing the Windows accent theme.
 * Windows 11 supports explicit caption/text/border colors; older Windows versions
 * retain the supported immersive dark-frame setting when those attributes fail. */
int zsr_style_title_bar(void *window)
{
    HWND hwnd = (HWND)window;
    BOOL dark = TRUE;
    COLORREF caption = RGB(0, 0, 0);
    COLORREF text = RGB(255, 255, 255);
    COLORREF border = RGB(170, 170, 170);
    if (!hwnd) return 0;
    /* DWMWA_USE_IMMERSIVE_DARK_MODE, BORDER_COLOR, CAPTION_COLOR, TEXT_COLOR. */
    DwmSetWindowAttribute(hwnd, 20, &dark, sizeof(dark));
    DwmSetWindowAttribute(hwnd, 34, &border, sizeof(border));
    HRESULT caption_result = DwmSetWindowAttribute(hwnd, 35, &caption, sizeof(caption));
    DwmSetWindowAttribute(hwnd, 36, &text, sizeof(text));
    SetWindowPos(hwnd, NULL, 0, 0, 0, 0,
                 SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED);
    return SUCCEEDED(caption_result);
}
#endif
