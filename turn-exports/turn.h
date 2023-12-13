#include <stdint.h>

#ifdef __cplusplus

#include <string>
#include <vector>
#include <stdexcept>
#include <functional>

#endif

typedef void (*GetPasswordCallback)(char* ret, void* ctx);
typedef void (*ProcessCallback)(bool is_success, ProcessResult* ret, void* ctx);

typedef struct
{
    void (*get_password)(char* addr, char* name, GetPasswordCallback callback, void* ctx);
    void (*allocated)(char* addr, char* name, uint16_t port);
    void (*binding)(char* addr);
    void (*channel_bind)(char* addr, char* name, uint16_t channel);
    void (*create_permission)(char* addr, char* name, char* relay);
    void (*refresh)(char* addr, char* name, uint32_t time);
    void (*abort)(char* addr, char* name);
} Observer;

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

typedef void* Service;
typedef void* Processor;

extern "C" Service crate_turn_service(char* realm, char** externals, size_t externals_len, Observer observer);
extern "C" void drop_turn_service(Service service);
extern "C" Processor get_processor(Service service, char* interface, char* external);
extern "C" void drop_processor(Processor processor);
extern "C" void process(Processor processor, uint8_t * buf, size_t buf_len, char* addr, ProcessCallback callback, void* ctx);

#ifdef __cplusplus

//typedef struct
//{
//    void (*get_password)(char* addr, char* name, GetPasswordCallback callback, void* ctx);
//    void (*allocated)(char* addr, char* name, uint16_t port);
//    void (*binding)(char* addr);
//    void (*channel_bind)(char* addr, char* name, uint16_t channel);
//    void (*create_permission)(char* addr, char* name, char* relay);
//    void (*refresh)(char* addr, char* name, uint32_t time);
//    void (*abort)(char* addr, char* name);
//} Observer;

class TurnObserver
{
public:
    virtual void GetPassword(std::string addr, std::string name)
    {
    }
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

    void Process(uint8_t* buf, size_t buf_len, std::string addr, std::function<void(bool is_success, ProcessResult* ret)> callback)
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
    TurnService(std::string realm, std::vector<std::string> externals, Observer observer)
    {
        char* externals_[20];
        for (size_t i = 0; i < externals.size(); i++)
        {
            externals_[i] = const_cast<char*>(externals[i].c_str());
        }

        _service = crate_turn_service(const_cast<char*>(realm.c_str()), externals_, externals.size(), observer);
        if (_service == nullptr)
        {
            throw std::runtime_error("crate turn service is failed!");
        }
    }

    ~TurnService()
    {
        drop_turn_service(_service);
    }

    TurnProcessor GetProcessor(std::string interface, std::string external)
    {
        Processor processor = get_processor(_service, const_cast<char*>(interface.c_str()), const_cast<char*>(external.c_str()));
        if (processor == nullptr)
        {
            throw std::runtime_error("get turn processor is failed!");
        }

        return TurnProcessor(processor);
    }
private:
    Service _service;
};

#endif // __cplusplus
