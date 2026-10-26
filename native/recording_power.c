#ifdef _WIN32
#include <windows.h>

#ifndef PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION
#define PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION 0x4
#endif

static SRWLOCK policy_lock = SRWLOCK_INIT;
static unsigned int users;
static PROCESS_POWER_THROTTLING_STATE previous;
static BOOL changed;

/* Preserve existing policy bits while opting out of the requested throttling. */
static BOOL protect_process(HANDLE process, DWORD mask,
                            PROCESS_POWER_THROTTLING_STATE *saved)
{
    PROCESS_POWER_THROTTLING_STATE state = {0};
    state.Version = PROCESS_POWER_THROTTLING_CURRENT_VERSION;
    if (!GetProcessInformation(process, ProcessPowerThrottling, &state, sizeof(state)))
        return FALSE;
    if (saved)
        *saved = state;
    state.ControlMask |= mask;
    state.StateMask &= ~mask;
    return SetProcessInformation(process, ProcessPowerThrottling, &state, sizeof(state));
}

/* Keep the owning UI process's timer policy stable until the last recording ends. */
int zsr_recording_power_begin(void)
{
    AcquireSRWLockExclusive(&policy_lock);
    if (users++ == 0) {
        changed = protect_process(GetCurrentProcess(),
            PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION, &previous);
    }
    int result = changed != FALSE;
    ReleaseSRWLockExclusive(&policy_lock);
    return result;
}

/* Restore the original policy even when capture startup or finalization failed. */
void zsr_recording_power_end(void)
{
    AcquireSRWLockExclusive(&policy_lock);
    if (users && --users == 0 && changed) {
        SetProcessInformation(GetCurrentProcess(), ProcessPowerThrottling,
                              &previous, sizeof(previous));
        changed = FALSE;
    }
    ReleaseSRWLockExclusive(&policy_lock);
}

/* Protect only the encoder child without changing the system power plan. */
int zsr_recording_power_child(void *handle)
{
    DWORD mask = PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION |
                 PROCESS_POWER_THROTTLING_EXECUTION_SPEED;
    if (protect_process((HANDLE)handle, mask, NULL))
        return 1;
    /* Older Windows versions may support execution-speed policy but not timer policy. */
    return protect_process((HANDLE)handle, PROCESS_POWER_THROTTLING_EXECUTION_SPEED, NULL) ? 2 : 0;
}

/* Raise only the owned recording process one level; return a Windows error on failure. */
unsigned long zsr_recording_priority(void *handle)
{
    if (SetPriorityClass((HANDLE)handle, ABOVE_NORMAL_PRIORITY_CLASS))
        return 0;
    return GetLastError();
}
#endif
