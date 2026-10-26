#ifdef _WIN32
#include <windows.h>
#include <dwmapi.h>

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
