#include "mars_xlog_wrapper.h"

#include <string>
#include <vector>

#include "mars/xlog/appender.h"
#include "mars/xlog/xlogger_interface.h"

namespace {

mars::xlog::XLogConfig ToCppConfig(const mars_xlog_config_t* cfg) {
    mars::xlog::XLogConfig out;
    if (!cfg) {
        return out;
    }
    out.mode_ = static_cast<mars::xlog::TAppenderMode>(cfg->mode);
    if (cfg->logdir) {
        out.logdir_ = cfg->logdir;
    }
    if (cfg->nameprefix) {
        out.nameprefix_ = cfg->nameprefix;
    }
    if (cfg->pub_key) {
        out.pub_key_ = cfg->pub_key;
    }
    out.compress_mode_ = static_cast<mars::xlog::TCompressMode>(cfg->compress_mode);
    out.compress_level_ = cfg->compress_level;
    if (cfg->cache_dir) {
        out.cachedir_ = cfg->cache_dir;
    }
    out.cache_days_ = cfg->cache_days;
    return out;
}

size_t CopyJoined(const std::vector<std::string>& items, char* buf, size_t len) {
    std::string joined;
    for (size_t i = 0; i < items.size(); ++i) {
        if (i != 0) {
            joined.push_back('\n');
        }
        joined.append(items[i]);
    }

    size_t required = joined.size() + 1;
    if (buf == nullptr || len == 0) {
        return required;
    }

    size_t to_copy = joined.size();
    if (to_copy >= len) {
        to_copy = len - 1;
    }
    if (to_copy > 0) {
        memcpy(buf, joined.data(), to_copy);
    }
    buf[to_copy] = '\0';
    return required;
}

}  // namespace

extern "C" {

uintptr_t mars_xlog_new_instance(const mars_xlog_config_t* cfg, int level) {
    if (cfg == nullptr) {
        return 0;
    }
    mars::xlog::XLogConfig cpp_cfg = ToCppConfig(cfg);
    mars::comm::XloggerCategory* category = mars::xlog::NewXloggerInstance(cpp_cfg, (TLogLevel)level);
    return reinterpret_cast<uintptr_t>(category);
}

uintptr_t mars_xlog_get_instance(const char* nameprefix) {
    mars::comm::XloggerCategory* category = mars::xlog::GetXloggerInstance(nameprefix);
    return reinterpret_cast<uintptr_t>(category);
}

void mars_xlog_release_instance(const char* nameprefix) {
    mars::xlog::ReleaseXloggerInstance(nameprefix);
}

void mars_xlog_appender_open(const mars_xlog_config_t* cfg, int level) {
    if (cfg == nullptr) {
        return;
    }
    mars::xlog::XLogConfig cpp_cfg = ToCppConfig(cfg);
    mars::xlog::appender_open(cpp_cfg);
    xlogger_SetLevel((TLogLevel)level);
}

void mars_xlog_appender_close(void) {
    mars::xlog::appender_close();
}

void mars_xlog_write(uintptr_t instance, const XLoggerInfo* info, const char* log) {
    mars::xlog::XloggerWrite(instance, info, log);
}

int mars_xlog_is_enabled(uintptr_t instance, int level) {
    return mars::xlog::IsEnabledFor(instance, (TLogLevel)level) ? 1 : 0;
}

int mars_xlog_get_level(uintptr_t instance) {
    return (int)mars::xlog::GetLevel(instance);
}

void mars_xlog_set_level(uintptr_t instance, int level) {
    mars::xlog::SetLevel(instance, (TLogLevel)level);
}

void mars_xlog_set_appender_mode(uintptr_t instance, int mode) {
    mars::xlog::SetAppenderMode(instance, (mars::xlog::TAppenderMode)mode);
}

void mars_xlog_flush(uintptr_t instance, int is_sync) {
    mars::xlog::Flush(instance, is_sync != 0);
}

void mars_xlog_flush_all(int is_sync) {
    mars::xlog::FlushAll(is_sync != 0);
}

void mars_xlog_set_console_log_open(uintptr_t instance, int is_open) {
    mars::xlog::SetConsoleLogOpen(instance, is_open != 0);
}

void mars_xlog_set_max_file_size(uintptr_t instance, long max_file_size) {
    mars::xlog::SetMaxFileSize(instance, max_file_size);
}

void mars_xlog_set_max_alive_time(uintptr_t instance, long alive_seconds) {
    mars::xlog::SetMaxAliveTime(instance, alive_seconds);
}

int mars_xlog_get_current_log_path(char* buf, unsigned int len) {
    return mars::xlog::appender_get_current_log_path(buf, len) ? 1 : 0;
}

int mars_xlog_get_current_log_cache_path(char* buf, unsigned int len) {
    return mars::xlog::appender_get_current_log_cache_path(buf, len) ? 1 : 0;
}

size_t mars_xlog_get_filepath_from_timespan(int timespan, const char* prefix, char* buf, size_t len) {
    std::vector<std::string> paths;
    if (!mars::xlog::appender_getfilepath_from_timespan(timespan, prefix, paths)) {
        if (buf && len > 0) {
            buf[0] = '\0';
        }
        return 1;
    }
    return CopyJoined(paths, buf, len);
}

size_t mars_xlog_make_logfile_name(int timespan, const char* prefix, char* buf, size_t len) {
    std::vector<std::string> paths;
    if (!mars::xlog::appender_make_logfile_name(timespan, prefix, paths)) {
        if (buf && len > 0) {
            buf[0] = '\0';
        }
        return 1;
    }
    return CopyJoined(paths, buf, len);
}

int mars_xlog_oneshot_flush(const mars_xlog_config_t* cfg, int* result_action) {
    if (cfg == nullptr) {
        return 0;
    }
    mars::xlog::XLogConfig cpp_cfg = ToCppConfig(cfg);
    mars::xlog::TFileIOAction action = mars::xlog::TFileIOAction::kActionNone;
    mars::xlog::appender_oneshot_flush(cpp_cfg, &action);
    if (result_action) {
        *result_action = static_cast<int>(action);
    }
    return 1;
}

const char* mars_xlog_dump(const void* buffer, size_t len) {
    return xlogger_dump(buffer, len);
}

const char* mars_xlog_memory_dump(const void* buffer, size_t len) {
    return xlogger_memory_dump(buffer, len);
}

void mars_xlog_set_console_fun(int fun) {
#ifdef __APPLE__
    mars::xlog::appender_set_console_fun((mars::xlog::TConsoleFun)fun);
#else
    (void)fun;
#endif
}

}  // extern "C"
