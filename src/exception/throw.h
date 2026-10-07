#ifndef EXCEPTION_THROW_H
#define EXCEPTION_THROW_H
#include <stdio.h>
// Recoverable cold diagnostic. Synchronous stderr; never a hot-path reporter.
#define THROW(...) do { fprintf(stderr, "[vex] %s:%d: ", __FILE__, __LINE__); \
    fprintf(stderr, __VA_ARGS__); fputc('\n', stderr); } while (0)
#endif
