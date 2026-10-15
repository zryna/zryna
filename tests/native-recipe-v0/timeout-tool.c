/* Trusted Linux x86-64 negative fixture, deliberately not a C compiler.
 * It ignores GCC arguments, creates no output, and normally exits 124 after
 * 35 monotonic seconds. The recipe's unchanged 30-second deadline must win.
 * No signal handlers, files, children, networking, or runtime library calls.
 */
_Static_assert(sizeof(long) == 8 && sizeof(void *) == 8,
               "Linux x86-64 LP64 only");

struct timespec64 {
    long seconds;
    long nanoseconds;
};
_Static_assert(sizeof(struct timespec64) == 16, "Linux timespec layout");

static long syscall3(long number, long first, long second, long third)
{
    long result;
    __asm__ volatile("syscall"
                     : "=a"(result)
                     : "a"(number), "D"(first), "S"(second), "d"(third)
                     : "rcx", "r11", "memory", "cc");
    return result;
}

__attribute__((noreturn)) static void finish(int status)
{
    syscall3(231, status, 0, 0); /* exit_group */
    for (;;) {} /* exit_group does not return on the supported kernel. */
}

static int valid_time(const struct timespec64 *time)
{
    return time->seconds >= 0 && time->nanoseconds >= 0 &&
           time->nanoseconds < 1000000000L;
}

static int earlier(const struct timespec64 *left,
                   const struct timespec64 *right)
{
    return left->seconds < right->seconds ||
           (left->seconds == right->seconds &&
            left->nanoseconds < right->nanoseconds);
}

__attribute__((noreturn)) void timeout_start(void)
{
    struct timespec64 previous = {0, 0};
    if (syscall3(228, 1, (long)&previous, 0) != 0 ||
        !valid_time(&previous) || previous.seconds > 0x7fffffffffffffffL - 35)
        finish(125);
    const struct timespec64 deadline = {
        previous.seconds + 35, previous.nanoseconds
    };

    /* Recompute the remaining interval after every interruption. A finite
     * retry limit fails closed instead of extending the intended deadline.
     * Scheduler latency is outside this ordinary fixture's timing guarantee.
     */
    for (unsigned int attempt = 0; attempt < 1024; ++attempt) {
        struct timespec64 now = {0, 0};
        if (syscall3(228, 1, (long)&now, 0) != 0 ||
            !valid_time(&now) || earlier(&now, &previous))
            finish(125);
        if (!earlier(&now, &deadline))
            finish(124);
        previous = now;
        struct timespec64 remaining = {
            deadline.seconds - now.seconds,
            deadline.nanoseconds - now.nanoseconds
        };
        if (remaining.nanoseconds < 0) {
            --remaining.seconds;
            remaining.nanoseconds += 1000000000L;
        }
        long result = syscall3(35, (long)&remaining, 0, 0); /* nanosleep */
        if (result != 0 && result != -4) /* EINTR */
            finish(125);
    }
    finish(125);
}

/* ELF entry is not a C function call. Establish the SysV call alignment. */
__asm__(".global _start\n.type _start,@function\n_start:\n"
        "andq $-16,%rsp\ncall timeout_start\nud2\n"
        ".size _start,.-_start\n");
