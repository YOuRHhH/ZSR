#include "picker.h"
#if defined(_WIN32)
#include <stdlib.h>

static POINT clamp(ZsrPicker *p, POINT point) {
    point.x = max(0, min(p->display.width, point.x));
    point.y = max(0, min(p->display.height, point.y));
    return point;
}

void zsr_picker_down(ZsrPicker *p, POINT point) {
    p->pressed = PtInRect(&p->confirm, point) ? 1 : PtInRect(&p->cancel, point) ? 2 : 0;
    if (PtInRect(&p->toolbar, point)) return;
    point = clamp(p, point);
    RECT r = p->selection;
    POINT corners[] = {{r.left,r.top},{r.right,r.top},{r.right,r.bottom},{r.left,r.bottom}};
    for (int i=0; i<4; ++i) {
        if (abs(point.x-corners[i].x)<=12 && abs(point.y-corners[i].y)<=12) {
            p->drag = 1; p->anchor = corners[(i+2)%4]; return;
        }
    }
    if (PtInRect(&r, point)) {
        p->drag = 2;
        p->offset = (POINT){point.x-r.left, point.y-r.top};
        p->size = (POINT){r.right-r.left, r.bottom-r.top};
    } else {
        p->drag = 1; p->anchor = point;
    }
}

void zsr_picker_move(ZsrPicker *p, POINT point) {
    if (!p->drag) return;
    point = clamp(p, point);
    RECT r;
    if (p->drag == 1) {
        r = (RECT){min(point.x,p->anchor.x), min(point.y,p->anchor.y),
                   max(point.x,p->anchor.x), max(point.y,p->anchor.y)};
    } else {
        LONG x = max(0,min(p->display.width-p->size.x,point.x-p->offset.x));
        LONG y = max(0,min(p->display.height-p->size.y,point.y-p->offset.y));
        r = (RECT){x,y,x+p->size.x,y+p->size.y};
    }
    if (!EqualRect(&r, &p->selection)) { p->selection = r; p->dirty = 1; }
}

void zsr_picker_up(ZsrPicker *p, POINT point) {
    zsr_picker_move(p, point);
    p->drag = 0;
    if (p->pressed == 1 && PtInRect(&p->confirm, point)
        && p->selection.right-p->selection.left >= 2
        && p->selection.bottom-p->selection.top >= 2) p->result = 1;
    if (p->pressed == 2 && PtInRect(&p->cancel, point)) p->result = 2;
    p->pressed = 0;
}
#endif
