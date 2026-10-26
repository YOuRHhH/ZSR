#ifndef ZSR_PICKER_H
#define ZSR_PICKER_H
#if defined(_WIN32)
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdint.h>
#include "display.h"
typedef struct {
    HWND window;
    HDC dc;
    HBITMAP bitmap;
    HGDIOBJ previous;
    HFONT font;
    uint32_t *pixels;
    ZsrDisplay display;
    RECT selection, toolbar, confirm, cancel;
    RECT painted;
    POINT anchor, offset, size;
    int drag, result, dirty, pressed;
    int has_frame;
    const wchar_t *instructions, *confirm_text, *cancel_text;
} ZsrPicker;
int zsr_picker_buffer(ZsrPicker *p);
void zsr_picker_free(ZsrPicker *p);
int zsr_picker_present(ZsrPicker *p);
void zsr_picker_draw(ZsrPicker *p);
void zsr_picker_down(ZsrPicker *p, POINT point);
void zsr_picker_move(ZsrPicker *p, POINT point);
void zsr_picker_up(ZsrPicker *p, POINT point);
#endif
#endif
