/* Tiny stand-in for the three kernel32 timer calls that Tools.HiResTimer P/Invokes.
   Mono on Linux has no Kernel32.dll, so every GrblFile.Load* (they read HiResTimer) would die.
   Mapped in via MONO_CONFIG dllmap by run.sh. Timer values never reach the golden output. */
#include <stdint.h>
#include <time.h>

int QueryPerformanceFrequency(int64_t *f) { *f = 1000000000LL; return 1; }

int QueryPerformanceCounter(int64_t *c)
{
	struct timespec ts;
	clock_gettime(CLOCK_MONOTONIC, &ts);
	*c = (int64_t)ts.tv_sec * 1000000000LL + ts.tv_nsec;
	return 1;
}

int GetTickCount(void)
{
	struct timespec ts;
	clock_gettime(CLOCK_MONOTONIC, &ts);
	return (int)(((int64_t)ts.tv_sec * 1000LL + ts.tv_nsec / 1000000LL) & 0xFFFFFFFF);
}
