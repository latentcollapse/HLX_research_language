#ifndef TEST_FFI_LIB_H
#define TEST_FFI_LIB_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// HLX FFI Exports
int64_t add(int64_t arg0, int64_t arg1);
int64_t multiply(int64_t arg0, int64_t arg1);

#ifdef __cplusplus
}
#endif

#endif // TEST_FFI_LIB_H
