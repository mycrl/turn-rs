//
//  turn.h
//  turn-lib
//
//  Created by Mr.Panda on 2023/12/16.
//

#ifndef LIB_TURN__H
#define LIB_TURN__H
#pragma once

#include <stdint.h>

#ifdef __cplusplus
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>
#endif

typedef enum
{
    Msg,
    Channel,
} StunClass;

typedef struct
{
    uint8_t* data;
    size_t data_len;
    StunClass kind;
    char* relay;
    char* interface;
} Response;

typedef enum
{
    InvalidInput,
    UnsupportedIpFamily,
    ShaFailed,
    NotIntegrity,
    IntegrityFailed,
    NotCookie,
    UnknownMethod,
    FatalError,
    Utf8Error,
} StunError;

typedef union
{
    Response response;
    StunError error;
} Result;

typedef struct
{
    bool is_success;
    Result result;
} ProcessRet;

typedef struct
{
    char* (*get_password)(char* addr, char* name, void* ctx);
    void (*allocated)(char* addr, char* name, uint16_t port, void* ctx);
    void (*binding)(char* addr, void* ctx);
    void (*channel_bind)(char* addr, char* name, uint16_t channel, void* ctx);
    void (*create_permission)(char* addr, char* name, char* relay, void* ctx);
    void (*refresh)(char* addr, char* name, uint32_t time, void* ctx);
    void (*abort)(char* addr, char* name, void* ctx);
} Observer;

typedef void* Service;
typedef void* Processor;

extern "C" Service crate_turn_service(char* realm,
                                      char** externals,
                                      size_t externals_len,
                                      Observer observer,
                                      void* ctx);

extern "C" void drop_turn_service(Service service);

extern "C" Processor get_processor(Service service,
                                   char* interface,
                                   char* external);

extern "C" void drop_processor(Processor processor);

extern "C" ProcessRet * process(Processor processor,
                                uint8_t * buf,
                                size_t buf_len,
                                char* addr);

extern "C" void drop_process_ret(ProcessRet * ret);

extern "C" const char* stun_err_into_str(StunError kind)
{
    switch (kind)
    {
        case StunError::InvalidInput:
            return ("InvalidInput");
            break;
        case StunError::UnsupportedIpFamily:
            return ("UnsupportedIpFamily");
            break;
        case StunError::ShaFailed:
            return ("ShaFailed");
            break;
        case StunError::NotIntegrity:
            return ("NotIntegrity");
            break;
        case StunError::IntegrityFailed:
            return ("IntegrityFailed");
            break;
        case StunError::NotCookie:
            return ("NotCookie");
            break;
        case StunError::UnknownMethod:
            return ("UnknownMethod");
            break;
        case StunError::FatalError:
            return ("FatalError");
            break;
        case StunError::Utf8Error:
            return ("Utf8Error");
            break;
        default:
            break;
    }
}

#ifdef __cplusplus
class TurnObserver
{
public:
    virtual std::optional<std::string> GetPassword(std::string& addr,
                                                   std::string& name)
    {
        return (std::nullopt);
    }

    virtual void Allocated(std::string& addr, std::string& name, uint16_t port)
    {
    }

    virtual void Binding(std::string& addr)
    {
    }

    virtual void ChannelBind(std::string& addr,
                             std::string& name,
                             uint16_t channel)
    {
    }

    virtual void CreatePermission(std::string& addr,
                                  std::string& name,
                                  std::string& relay)
    {
    }

    virtual void Refresh(std::string& addr, std::string& name, uint32_t time)
    {
    }

    virtual void Abort(std::string& addr, std::string& name)
    {
    }
};

namespace StaticObserver
{
    char* get_password(char* addr, char* name, void* ctx)
    {
        auto observer = (TurnObserver*)ctx;
        auto addr_ = std::move(std::string(addr));
        auto name_ = std::move(std::string(name));
        auto ret = observer->GetPassword(addr_, name_);
        return (ret.has_value() ? const_cast<char*>(ret.value().c_str()) : nullptr);
    }

    void allocated(char* addr, char* name, uint16_t port, void* ctx)
    {
        auto observer = (TurnObserver*)ctx;
        auto addr_ = std::move(std::string(addr));
        auto name_ = std::move(std::string(name));
        observer->Allocated(addr_, name_, port);
    }

    void binding(char* addr, void* ctx)
    {
        auto observer = (TurnObserver*)ctx;
        auto addr_ = std::move(std::string(addr));
        observer->Binding(addr_);
    }

    void channel_bind(char* addr, char* name, uint16_t channel, void* ctx)
    {
        auto observer = (TurnObserver*)ctx;
        auto addr_ = std::move(std::string(addr));
        auto name_ = std::move(std::string(name));
        observer->ChannelBind(addr_, name_, channel);
    }

    void create_permission(char* addr, char* name, char* relay, void* ctx)
    {
        auto observer = (TurnObserver*)ctx;
        auto addr_ = std::move(std::string(addr));
        auto name_ = std::move(std::string(name));
        auto relay_ = std::move(std::string(relay));
        observer->CreatePermission(addr_, name_, relay_);
    }

    void refresh(char* addr, char* name, uint32_t time, void* ctx)
    {
        auto observer = (TurnObserver*)ctx;
        auto addr_ = std::move(std::string(addr));
        auto name_ = std::move(std::string(name));
        observer->Refresh(addr_, name_, time);
    }

    void abort(char* addr, char* name, void* ctx)
    {
        auto observer = (TurnObserver*)ctx;
        auto addr_ = std::move(std::string(addr));
        auto name_ = std::move(std::string(name));
        observer->Abort(addr_, name_);
    }

    static Observer Objects = { get_password, allocated, binding, channel_bind,
                               create_permission, refresh, abort };
} // namespace StaticObserver

class TurnProcessor
{
public:
    class Results
    {
    public:
        ProcessRet* Ret;

        Results(ProcessRet* ret) : Ret(ret)
        {
        }

        ~Results()
        {
            drop_process_ret(Ret);
        }
    };

    TurnProcessor(Processor processor) : _processor(processor)
    {
    }

    ~TurnProcessor()
    {
        drop_processor(_processor);
    }

    std::unique_ptr<Results> Process(uint8_t* buf,
                                     size_t buf_len,
                                     std::string& addr)
    {
        ProcessRet* ret = process(_processor,
                                  buf,
                                  buf_len,
                                  const_cast<char*>(addr.c_str()));
        return (ret == nullptr ? nullptr : std::make_unique<Results>(ret));
    }

private:
    Processor _processor;
};

class TurnService
{
public:
    TurnService(std::string& realm, std::vector<std::string> externals,
                TurnObserver* observer)
    {
        char* externals_[20];
        for (size_t i = 0; i < externals.size(); i++)
        {
            externals_[i] = const_cast<char*>(externals[i].c_str());
        }

        _service = crate_turn_service(const_cast<char*>(realm.c_str()),
                                      externals_,
                                      externals.size(),
                                      StaticObserver::Objects,
                                      observer);
        if (_service == nullptr)
        {
            throw std::runtime_error("crate turn service is failed!");
        }
    }

    ~TurnService()
    {
        drop_turn_service(_service);
    }

    TurnProcessor* GetProcessor(std::string& interface, std::string& external)
    {
        Processor processor = get_processor(_service,
                                            const_cast<char*>(interface.c_str()),
                                            const_cast<char*>(external.c_str()));
        if (processor == nullptr)
        {
            return (nullptr);
        }

        return (new TurnProcessor(processor));
    }

private:
    Service _service;
};
#endif // __cplusplus

#endif // LIB_TURN__H