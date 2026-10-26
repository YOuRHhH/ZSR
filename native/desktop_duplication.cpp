#include "display.h"
#include <windows.h>
#include <dxgi.h>

/* Match the physical display to its DXGI adapter/output; monitor enumeration orders differ. */
extern "C" int zsr_duplication_output(const ZsrDisplay *display, unsigned int *adapter_id, unsigned int *output_id)
{
    if (!display || !adapter_id || !output_id)
        return 0;
    IDXGIFactory1 *factory = nullptr;
    if (FAILED(CreateDXGIFactory1(__uuidof(IDXGIFactory1), reinterpret_cast<void **>(&factory))))
        return 0;
    int found = 0;
    IDXGIAdapter1 *adapter = nullptr;
    for (UINT a = 0; !found && factory->EnumAdapters1(a, &adapter) == S_OK; ++a) {
        IDXGIOutput *output = nullptr;
        for (UINT o = 0; adapter->EnumOutputs(o, &output) == S_OK; ++o) {
            DXGI_OUTPUT_DESC desc = {};
            if (SUCCEEDED(output->GetDesc(&desc)) && desc.AttachedToDesktop &&
                desc.DesktopCoordinates.left == display->x && desc.DesktopCoordinates.top == display->y &&
                desc.DesktopCoordinates.right - desc.DesktopCoordinates.left == display->width &&
                desc.DesktopCoordinates.bottom - desc.DesktopCoordinates.top == display->height) {
                *adapter_id = a;
                *output_id = o;
                found = 1;
            }
            output->Release();
            if (found) break;
        }
        adapter->Release();
    }
    factory->Release();
    return found;
}
