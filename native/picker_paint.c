#include "picker.h"
#if defined(_WIN32)
#include <stdio.h>

static void fill(ZsrPicker *p, RECT r, uint32_t color) {
    r.left = max(0, r.left); r.top = max(0, r.top);
    r.right = min(p->display.width, r.right); r.bottom = min(p->display.height, r.bottom);
    for (LONG y = r.top; y < r.bottom; ++y)
        for (LONG x = r.left; x < r.right; ++x)
            p->pixels[(size_t)y * p->display.width + x] = color;
}

int zsr_picker_buffer(ZsrPicker *p) {
    BITMAPINFO info = {0};
    info.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    info.bmiHeader.biWidth = p->display.width;
    info.bmiHeader.biHeight = -p->display.height;
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;
    p->dc = CreateCompatibleDC(NULL);
    if (!p->dc) return 0;
    p->bitmap = CreateDIBSection(p->dc, &info, DIB_RGB_COLORS, (void **)&p->pixels, NULL, 0);
    if (!p->bitmap) return 0;
    p->previous = SelectObject(p->dc, p->bitmap);
    UINT dpi = GetDpiForWindow(p->window);
    int unit = MulDiv(16, (int)dpi, 96);
    int width = min(p->display.width - 2 * unit, 43 * unit);
    p->toolbar = (RECT){unit, unit, unit + width, 7 * unit};
    p->confirm = (RECT){unit + width - 23 * unit, 4 * unit, unit + width - 11 * unit, 6 * unit};
    p->cancel = (RECT){unit + width - 10 * unit, 4 * unit, unit + width - unit, 6 * unit};
    p->font = CreateFontW(-unit, 0, 0, 0, FW_NORMAL, FALSE, FALSE, FALSE,
        DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY,
        DEFAULT_PITCH, L"Segoe UI");
    fill(p, (RECT){0, 0, p->display.width, p->display.height}, 0x01000000);
    return p->font != NULL;
}

static void label(ZsrPicker *p, RECT r, const wchar_t *text, UINT flags) {
    DrawTextW(p->dc, text, -1, &r, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | flags);
}

static void outline(ZsrPicker *p, RECT r, uint32_t color, uint32_t handle) {
    fill(p, (RECT){r.left, r.top, r.right, r.top + 2}, color);
    fill(p, (RECT){r.left, r.bottom - 2, r.right, r.bottom}, color);
    fill(p, (RECT){r.left, r.top, r.left + 2, r.bottom}, color);
    fill(p, (RECT){r.right - 2, r.top, r.right, r.bottom}, color);
    LONG xs[] = {r.left, r.right}, ys[] = {r.top, r.bottom};
    for (int x = 0; x < 2; ++x) for (int y = 0; y < 2; ++y)
        fill(p, (RECT){xs[x]-4, ys[y]-4, xs[x]+4, ys[y]+4}, handle);
}

void zsr_picker_draw(ZsrPicker *p) {
    /* Alpha=1 catches input with an imperceptible tint. Reuse the desktop-sized
     * bitmap: only erase the previous outline and redraw the frame/toolbar. */
    if (p->has_frame) outline(p, p->painted, 0x01000000, 0x01000000);
    RECT r = p->selection;
    outline(p, r, 0xff5ab9ff, 0xffffffff);
    p->painted = r; p->has_frame = 1;
    fill(p, p->toolbar, 0xff202020);
    fill(p, p->confirm, 0xff454545);
    fill(p, p->cancel, 0xff454545);
    HGDIOBJ old_font = SelectObject(p->dc, p->font);
    SetBkMode(p->dc, TRANSPARENT);
    SetTextColor(p->dc, RGB(245, 245, 245));
    RECT instruction = p->toolbar;
    instruction.left += 10; instruction.bottom = p->confirm.top;
    label(p, instruction, p->instructions, DT_LEFT | DT_END_ELLIPSIS);
    label(p, p->confirm, p->confirm_text, DT_CENTER);
    label(p, p->cancel, p->cancel_text, DT_CENTER);
    wchar_t dimensions[64];
    swprintf_s(dimensions, 64, L"%ld x %ld px", r.right-r.left, r.bottom-r.top);
    RECT dimension_rect = {instruction.left, p->confirm.top, p->confirm.left, p->confirm.bottom};
    label(p, dimension_rect, dimensions, DT_LEFT);
    SelectObject(p->dc, old_font);
    GdiFlush();
    /* GDI text doesn't preserve alpha; the toolbar is fully opaque. */
    for (LONG y = max(0,p->toolbar.top); y < min(p->display.height,p->toolbar.bottom); ++y)
        for (LONG x = max(0,p->toolbar.left); x < min(p->display.width,p->toolbar.right); ++x)
            p->pixels[(size_t)y*p->display.width+x] |= 0xff000000;
}

int zsr_picker_present(ZsrPicker *p) {
    zsr_picker_draw(p);
    POINT origin = {p->display.x, p->display.y}, source = {0,0};
    SIZE size = {p->display.width, p->display.height};
    BLENDFUNCTION blend = {AC_SRC_OVER, 0, 255, AC_SRC_ALPHA};
    /* One atomic bitmap update: geometry and pixels never arrive separately. */
    return UpdateLayeredWindow(p->window, NULL, &origin, &size, p->dc,
        &source, 0, &blend, ULW_ALPHA) != 0;
}

void zsr_picker_free(ZsrPicker *p) {
    if (p->previous) SelectObject(p->dc, p->previous);
    if (p->bitmap) DeleteObject(p->bitmap);
    if (p->font) DeleteObject(p->font);
    if (p->dc) DeleteDC(p->dc);
}
#endif
