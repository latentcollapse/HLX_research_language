/**
 * @file hlx.h
 * @brief C API for the HLX Runtime
 */

#ifndef HLX_H
#define HLX_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stddef.h>

typedef struct HlxHandle hlx_t;

hlx_t *hlx_open();
void    hlx_close(hlx_t *h);
int     hlx_reset(hlx_t *h);  /* Reset VM to fresh state, wiping all memory */
int     hlx_set_search_path(hlx_t *h, const char *path);
int     hlx_compile_source(hlx_t *h, const char *source);
int     hlx_compile_file(hlx_t *h, const char *path);
char   *hlx_run(hlx_t *h);
char   *hlx_call(hlx_t *h, const char *func, const char *args_json);
void    hlx_free_string(char *s);
const char *hlx_errmsg(hlx_t *h);
char       *hlx_list_functions(hlx_t *h);
const char *hlx_version();

#ifdef __cplusplus
}
#endif

#endif
