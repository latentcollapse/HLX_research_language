#include <stdio.h>
#include "test_ffi_lib.h"

int main() {
    int64_t sum = add(100, 200);
    printf("add(100, 200) = %ld\n", sum);

    int64_t product = multiply(7, 8);
    printf("multiply(7, 8) = %ld\n", product);

    // Test chaining
    int64_t result = multiply(add(10, 5), 3);
    printf("multiply(add(10, 5), 3) = %ld\n", result);

    return 0;
}
