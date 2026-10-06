#include "io/process_spawn.h"

#include <signal.h>
#include <spawn.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

#include "nio/mem.h"
#include "oop/type.h"
#include "annotation/definition.h"
#include "annotation/overview.h"

extern char **environ;

;;DEFINITION
/**
 * ============================================================================
 * DEFINITION: ProcessSpawn
 * ============================================================================
 * A bounded child-process job table for the R1 leaf layer: fixed slots
 * (PROCESS_SPAWN_JOBS_MAX), per-job pid/exit/done rows, table-level mirrors,
 * a cancel flag, and a timeout — zero steady-state allocation, no threads.
 * Spawns via posix_spawnp (never system(), never a blocking waitpid, never
 * UINT64_MAX); poll reaps with WNOHANG in ~1ms slices up to a 100ms budget,
 * and cancel raises the flag and SIGTERMs unfinished jobs. This is the
 * decoder-binary seam: callers such as graphvex FrameImporter spawn external
 * decoders through this shape instead of popen or libav links.
 * ============================================================================
 */

;;OVERVIEW
/**
 * ============================================================================
 * CLASS: ProcessSpawn (io/process_spawn.c)
 * ============================================================================
 * A bounded child-process job table for the R1 leaf layer. Fixed slots
 * (PROCESS_SPAWN_JOBS_MAX), per-job pid/exit/done rows, table-level
 * mirrors, cancel flag, and timeout — zero steady-state allocation, no
 * threads. Spawns via posix_spawnp (never system(), never a blocking
 * waitpid, never UINT64_MAX): ProcessSpawn_poll reaps with WNOHANG in
 * ~1ms slices up to budgetNs clamped to PROCESS_SPAWN_POLL_MAX_NS
 * (100ms, Rule 27); ProcessSpawn_cancel raises the flag and SIGTERMs
 * unfinished jobs. The decoder-binary seam: callers (e.g. graphvex
 * FrameImporter) spawn external decoders through this shape instead of
 * popen/libav links.
 *
 * STRUCT FIELDS (Mirroring io/process_spawn.h — exactly this file's class):
 * ----------------------------------------------------------------------------
 *   ProcessSpawn {
 *     int32_t pid;                                // most recent child pid
 *     uint64_t timeoutMs;                         // default spawn budget (ms)
 *     int32_t exitCode;                           // last reaped exit code
 *     bool cancelFlag;                            // cancel raised; poll degrades
 *     ProcessSpawnJob jobs[PROCESS_SPAWN_JOBS_MAX]; // fixed table, no alloc
 *     uint32_t count;                             // live jobs (0..JOBS_MAX)
 *   }
 *
 * SLOT RECORD (ProcessSpawnJob — Rule 3 co-location, zero behavior):
 * ----------------------------------------------------------------------------
 *   int32_t pid;       // child pid (> 0 tracked, 0 = slot free)
 *   int32_t exitCode;  // WEXITSTATUS / -signal once done
 *   bool done;         // true once waitpid has reaped the child
 *
 * FUNCTION REGISTRY:
 * ----------------------------------------------------------------------------
 * Constructors:
 *   - ProcessSpawn()              : ProcessSpawn_0()
 *   - ProcessSpawn(timeoutMs)     : ProcessSpawn_1(timeoutMs)
 *
 * Core Functions:
 *   - ProcessSpawn_free(self)
 *   - ProcessSpawn_spawn(argv, timeoutMs, dest)
 *   - ProcessSpawn_poll(self, budgetNs)
 *   - ProcessSpawn_cancel(self)
 *
 * Setters:
 *   - ProcessSpawn_setPid(self, pid)
 *   - ProcessSpawn_setTimeoutMs(self, timeoutMs)
 *   - ProcessSpawn_setExitCode(self, exitCode)
 *   - ProcessSpawn_setCancelFlag(self, cancelFlag)
 *
 * Getters:
 *   - ProcessSpawn_getPid(self)
 *   - ProcessSpawn_getTimeoutMs(self)
 *   - ProcessSpawn_getExitCode(self)
 *   - ProcessSpawn_isCancelFlag(self)
 *   - ProcessSpawn_getCount(self)
 *   - ProcessSpawn_getJobPid(self, i)
 *   - ProcessSpawn_getJobExitCode(self, i)
 *   - ProcessSpawn_isJobDone(self, i)
 * ============================================================================
 */

// io/process_spawn.c — ProcessSpawn port. posix_spawnp launch, WNOHANG
// reap slices, SIGTERM cancel. No system(), no blocking wait, no threads.

static uint64_t processSpawnNowNs(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t) ts.tv_sec * 1000000000ULL + (uint64_t) ts.tv_nsec;
}

static void processSpawnSleepSlice(void) {
    struct timespec slice;
    slice.tv_sec = 0;
    slice.tv_nsec = 1000000L;
    nanosleep(&slice, nullptr);
}

static ProcessSpawn *processSpawnCreate(uint64_t timeoutMs) {
    ProcessSpawn *self = (ProcessSpawn*) Memory_alloc(TYPE_PROCESS_SPAWN_SINGLETON, sizeof(ProcessSpawn));
    if (!self)
        return nullptr;
    (*self).pid = 0;
    (*self).timeoutMs = timeoutMs;
    (*self).exitCode = 0;
    (*self).cancelFlag = false;
    for (uint32_t i = 0; i < PROCESS_SPAWN_JOBS_MAX; i++) {
        ProcessSpawnJob *job = &(*self).jobs[i];
        (*job).pid = 0;
        (*job).exitCode = 0;
        (*job).done = true;
    }
    (*self).count = 0;
    return self;
}

static bool processSpawnReapOnce(ProcessSpawn *self) {
    bool allDone = true;
    for (uint32_t i = 0; i < PROCESS_SPAWN_JOBS_MAX; i++) {
        ProcessSpawnJob *job = &(*self).jobs[i];
        if ((*job).pid == 0 || (*job).done)
            continue;
        int status = 0;
        pid_t got = waitpid((pid_t)(*job).pid, &status, WNOHANG);
        if (got == 0) {
            allDone = false;
            continue;
        }
        if (got < 0) {
            (*job).exitCode = -1;
            (*job).done = true;
            continue;
        }
        if (WIFEXITED(status))
            (*job).exitCode = WEXITSTATUS(status);
        else if (WIFSIGNALED(status))
            (*job).exitCode = -WTERMSIG(status);
        else
            (*job).exitCode = -1;
        (*job).done = true;
        (*self).exitCode = (*job).exitCode;
    }
    return allDone;
}

// CONSTRUCTORS
ProcessSpawn *ProcessSpawn_0(void) {
    return processSpawnCreate(0);
}

ProcessSpawn *ProcessSpawn_1(uint64_t timeoutMs) {
    return processSpawnCreate(timeoutMs);
}

// CORE FUNCTIONS
void ProcessSpawn_free(ProcessSpawn *self) {
    if (!self)
        return;
    Memory_free(self);
}

bool ProcessSpawn_spawn(const char *const *argv, uint64_t timeoutMs,
                        ProcessSpawn *dest) {
    if (!argv || !(*argv) || !dest)
        return false;
    if ((*dest).cancelFlag)
        return false;
    ProcessSpawnJob *slot = nullptr;
    for (uint32_t i = 0; i < PROCESS_SPAWN_JOBS_MAX; i++) {
        ProcessSpawnJob *job = &(*dest).jobs[i];
        if ((*job).pid == 0 || (*job).done) {
            slot = job;
            break;
        }
    }
    if (!slot)
        return false;
    pid_t child = 0;
    char *const *args = (char *const*) argv;
    int rc = posix_spawnp(&child, (*argv), nullptr, nullptr, args, environ);
    if (rc != 0)
        return false;
    (*slot).pid = (int32_t) child;
    (*slot).exitCode = 0;
    (*slot).done = false;
    (*dest).pid = (int32_t) child;
    (*dest).timeoutMs = timeoutMs;
    (*dest).count += 1;
    return true;
}

bool ProcessSpawn_poll(ProcessSpawn *self, uint64_t budgetNs) {
    if (!self)
        return false;
    if (budgetNs > PROCESS_SPAWN_POLL_MAX_NS)
        budgetNs = PROCESS_SPAWN_POLL_MAX_NS;
    if ((*self).cancelFlag)
        return processSpawnReapOnce(self);
    uint64_t start = processSpawnNowNs();
    for (;;) {
        bool allDone = processSpawnReapOnce(self);
        if (allDone)
            return true;
        if ((*self).cancelFlag)
            return false;
        if (processSpawnNowNs() - start >= budgetNs)
            return false;
        processSpawnSleepSlice();
    }
}

void ProcessSpawn_cancel(ProcessSpawn *self) {
    if (!self)
        return;
    (*self).cancelFlag = true;
    for (uint32_t i = 0; i < PROCESS_SPAWN_JOBS_MAX; i++) {
        ProcessSpawnJob *job = &(*self).jobs[i];
        if ((*job).pid != 0 && !(*job).done)
            kill((pid_t)(*job).pid, SIGTERM);
    }
}

// SETTERS
void ProcessSpawn_setPid(ProcessSpawn *self, int32_t pid) {
    if (!self)
        return;
    (*self).pid = pid;
}

void ProcessSpawn_setTimeoutMs(ProcessSpawn *self, uint64_t timeoutMs) {
    if (!self)
        return;
    (*self).timeoutMs = timeoutMs;
}

void ProcessSpawn_setExitCode(ProcessSpawn *self, int32_t exitCode) {
    if (!self)
        return;
    (*self).exitCode = exitCode;
}

void ProcessSpawn_setCancelFlag(ProcessSpawn *self, bool cancelFlag) {
    if (!self)
        return;
    (*self).cancelFlag = cancelFlag;
}

// GETTERS
int32_t ProcessSpawn_getPid(const ProcessSpawn *self) {
    if (!self)
        return 0;
    return (*self).pid;
}

uint64_t ProcessSpawn_getTimeoutMs(const ProcessSpawn *self) {
    if (!self)
        return 0;
    return (*self).timeoutMs;
}

int32_t ProcessSpawn_getExitCode(const ProcessSpawn *self) {
    if (!self)
        return 0;
    return (*self).exitCode;
}

bool ProcessSpawn_isCancelFlag(const ProcessSpawn *self) {
    if (!self)
        return true;
    return (*self).cancelFlag;
}

uint32_t ProcessSpawn_getCount(const ProcessSpawn *self) {
    if (!self)
        return 0;
    return (*self).count;
}

int32_t ProcessSpawn_getJobPid(const ProcessSpawn *self, uint32_t i) {
    if (!self)
        return 0;
    if (i >= PROCESS_SPAWN_JOBS_MAX)
        return 0;
    const ProcessSpawnJob *job = &(*self).jobs[i];
    return (*job).pid;
}

int32_t ProcessSpawn_getJobExitCode(const ProcessSpawn *self, uint32_t i) {
    if (!self)
        return 0;
    if (i >= PROCESS_SPAWN_JOBS_MAX)
        return 0;
    const ProcessSpawnJob *job = &(*self).jobs[i];
    return (*job).exitCode;
}

bool ProcessSpawn_isJobDone(const ProcessSpawn *self, uint32_t i) {
    if (!self)
        return true;
    if (i >= PROCESS_SPAWN_JOBS_MAX)
        return true;
    const ProcessSpawnJob *job = &(*self).jobs[i];
    return (*job).done;
}
