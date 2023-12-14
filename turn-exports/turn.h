#include <stdint.h>

#ifdef __cplusplus

#include <string>
#include <vector>
#include <stdexcept>
#include <functional>
#include <optional>

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
} ProcessResult;

typedef void (*GetPasswordCallback)(char* ret, void* call_ctx);
typedef void (*ProcessCallback)(bool is_success, ProcessResult* ret, void* ctx);

typedef struct
{
    void (*get_password)(char* addr, char* name, GetPasswordCallback callback, void* ctx, void* call_ctx);
    void (*allocated)(char* addr, char* name, uint16_t port, void* ctx);
    void (*binding)(char* addr, void* ctx);
    void (*channel_bind)(char* addr, char* name, uint16_t channel, void* ctx);
    void (*create_permission)(char* addr, char* name, char* relay, void* ctx);
    void (*refresh)(char* addr, char* name, uint32_t time, void* ctx);
    void (*abort)(char* addr, char* name, void* ctx);
} Observer;

typedef void* Service;
typedef void* Processor;

extern "C" Service crate_turn_service(char* realm, char** externals, size_t externals_len, Observer observer, void* ctx);
extern "C" void drop_turn_service(Service service);
extern "C" Processor get_processor(Service service, char* interface, char* external);
extern "C" void drop_processor(Processor processor);
extern "C" void process(Processor processor, uint8_t * buf, size_t buf_len, char* addr, ProcessCallback callback, void* ctx);

#ifdef __cplusplus

class TurnObserver
{
public:
    virtual void GetPassword(std::string& addr,
                             std::string& name,
                             std::function<void(std::optional<std::string>) > callback)
    {
        callback(std::nullopt);
    }

    virtual void Allocated(std::string& addr,
                           std::string& name,
                           uint16_t port)
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

    virtual void Refresh(std::string& addr,
                         std::string& name,
                         uint32_t time)
    {
    }

    virtual void Abort(std::string& addr,
                       std::string& name)
    {
    }
};

namespace ObserverClass
{
    void get_password(char* addr, char* name, GetPasswordCallback callback, void* ctx, void* call_ctx)
    {
        auto observer = (TurnObserver*)ctx;
        auto addr_ = std::move(std::string(addr));
        auto name_ = std::move(std::string(name));
        observer->GetPassword(addr_,
                              name_,
                              [&](std::optional<std::string> ret)
                              {
                                  callback(ret.has_value() ? const_cast<char*>(ret.value().c_str()) : nullptr, call_ctx);
                              });
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

    const Observer OBSERVER = Observer{
        get_password,
        allocated,
        binding,
        channel_bind,
        create_permission,
        refresh,
        abort };
};

class TurnProcessor
{
public:
    TurnProcessor(Processor processor) : _processor(process)
    {
    }

    ~TurnProcessor()
    {
        drop_processor(_processor);
    }

    void Process(uint8_t* buf,
                 size_t buf_len,
                 std::string& addr,
                 std::function<void(bool is_success, ProcessResult* ret)> callback)
    {
        process(_processor, buf, buf_len, const_cast<char*>(addr.c_str()), Callback, &callback);
    }
private:
    Processor _processor;

    static void Callback(bool is_success, ProcessResult* ret, void* ctx)
    {
        auto callback = (std::function<void(bool is_success, ProcessResult * ret)>*)ctx;
        (*callback)(is_success, ret);
    }
};

class TurnService
{
public:
    TurnService(std::string& realm, std::vector<std::string> externals, TurnObserver* observer)
    {
        char* externals_[20];
        for (size_t i = 0; i < externals.size(); i++)
        {
            externals_[i] = const_cast<char*>(externals[i].c_str());
        }

        _service = crate_turn_service(const_cast<char*>(realm.c_str()),
                                      externals_,
                                      externals.size(),
                                      ObserverClass::OBSERVER,
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
            return nullptr;
        }

        return new TurnProcessor(processor);
    }
private:
    Service _service;
};

#endif // __cplusplus
