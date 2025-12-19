#ifndef MARS_XLOG_WRAPPER_H_
#define MARS_XLOG_WRAPPER_H_

#include <stddef.h>
#include <stdint.h>

#include "mars/comm/xlogger/xloggerbase.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct mars_xlog_config_t {
    int mode;             // mars::xlog::TAppenderMode
    const char* logdir;
    const char* nameprefix;
    const char* pub_key;
    int compress_mode;    // mars::xlog::TCompressMode
    int compress_level;
    const char* cache_dir;
    int cache_days;
} mars_xlog_config_t;

// instance lifecycle
uintptr_t mars_xlog_new_instance(const mars_xlog_config_t* cfg, int level);
uintptr_t mars_xlog_get_instance(const char* nameprefix);
void mars_xlog_release_instance(const char* nameprefix);

// global appender (default instance)
void mars_xlog_appender_open(const mars_xlog_config_t* cfg, int level);
void mars_xlog_appender_close(void);

// logging
void mars_xlog_write(uintptr_t instance, const XLoggerInfo* info, const char* log);
int mars_xlog_is_enabled(uintptr_t instance, int level);
int mars_xlog_get_level(uintptr_t instance);
void mars_xlog_set_level(uintptr_t instance, int level);

// controls
void mars_xlog_set_appender_mode(uintptr_t instance, int mode);
void mars_xlog_flush(uintptr_t instance, int is_sync);
void mars_xlog_flush_all(int is_sync);
void mars_xlog_set_console_log_open(uintptr_t instance, int is_open);
void mars_xlog_set_max_file_size(uintptr_t instance, long max_file_size);
void mars_xlog_set_max_alive_time(uintptr_t instance, long alive_seconds);

// paths
int mars_xlog_get_current_log_path(char* buf, unsigned int len);
int mars_xlog_get_current_log_cache_path(char* buf, unsigned int len);

// return required length (including trailing '\0') even if buf is NULL/len=0
size_t mars_xlog_get_filepath_from_timespan(int timespan, const char* prefix, char* buf, size_t len);
size_t mars_xlog_make_logfile_name(int timespan, const char* prefix, char* buf, size_t len);

// one-shot flush
int mars_xlog_oneshot_flush(const mars_xlog_config_t* cfg, int* result_action);

// dumps
const char* mars_xlog_dump(const void* buffer, size_t len);
const char* mars_xlog_memory_dump(const void* buffer, size_t len);

// iOS console control (no-op on non-Apple)
void mars_xlog_set_console_fun(int fun);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // MARS_XLOG_WRAPPER_H_
