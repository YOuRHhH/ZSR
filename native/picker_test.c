#include "picker.h"
#if defined(_WIN32)

/* Offscreen regression checks: production input and drawing, no screen capture,
 * no foreground changes, and no synthetic system mouse input. */
int32_t zsr_picker_verify(double *micros) {
    ZsrPicker p = {0};
    p.display = (ZsrDisplay){0,0,1920,1080,0};
    p.instructions = L"Drag to select"; p.confirm_text = L"Use area"; p.cancel_text = L"Cancel";
    p.window = CreateWindowExW(WS_EX_LAYERED, L"STATIC", L"ZSR offscreen test",
        WS_POPUP,0,0,1920,1080,NULL,NULL,GetModuleHandleW(NULL),NULL);
    if (!p.window) return 1;
    if (!zsr_picker_buffer(&p)) { zsr_picker_free(&p); DestroyWindow(p.window); return 2; }
    int error = 0;
    p.selection = (RECT){200,200,600,500};
    zsr_picker_down(&p, (POINT){1000,600});
    for (int i=1;i<=200;++i) zsr_picker_move(&p, (POINT){1000+i,600+i});
    zsr_picker_up(&p, (POINT){1200,800});
    RECT expected = {1000,600,1200,800};
    if (!EqualRect(&p.selection,&expected) || p.drag) error |= 4;
    zsr_picker_draw(&p);
    if (p.pixels[700*1920+1100] != 0x01000000) error |= 8;
    if (p.pixels[600*1920+1100] != 0xff5ab9ff) error |= 16;
    zsr_picker_down(&p, (POINT){1100,700});
    zsr_picker_move(&p, (POINT){1919,1079});
    zsr_picker_up(&p, (POINT){1919,1079});
    expected = (RECT){1720,880,1920,1080};
    if (!EqualRect(&p.selection,&expected)) error |= 32;
    zsr_picker_draw(&p);
    if (p.pixels[600*1920+1100] != 0x01000000) error |= 64; /* old border erased */
    zsr_picker_down(&p, (POINT){1720,880});
    zsr_picker_move(&p, (POINT){1500,700});
    zsr_picker_up(&p, (POINT){1500,700});
    expected = (RECT){1500,700,1920,1080};
    if (!EqualRect(&p.selection,&expected)) error |= 128;
    if (!zsr_picker_present(&p)) error |= 256; /* actual hidden layered-window API */
    LARGE_INTEGER start, end, frequency;
    QueryPerformanceFrequency(&frequency); QueryPerformanceCounter(&start);
    for (int i=0;i<100;++i) { p.selection.left = 1000+i; zsr_picker_draw(&p); }
    QueryPerformanceCounter(&end);
    *micros = (double)(end.QuadPart-start.QuadPart)*1000000.0/(double)frequency.QuadPart/100.0;
    POINT confirm = {p.confirm.left+2,p.confirm.top+2};
    zsr_picker_down(&p,confirm); zsr_picker_up(&p,confirm);
    if (p.result != 1) error |= 512;
    POINT cancel = {p.cancel.left+2,p.cancel.top+2};
    zsr_picker_down(&p,cancel); zsr_picker_up(&p,cancel);
    if (p.result != 2) error |= 1024;
    zsr_picker_free(&p); DestroyWindow(p.window);
    return error;
}
#endif
