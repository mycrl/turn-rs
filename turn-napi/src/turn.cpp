#include <memory>
#include "turn.h"
#include <napi.h>

enum JsTypes
{
    String,
    Number,
    Boolean,
    Object,
    Array,
    Buffer,
    Function,
};

bool args_checker(const Napi::CallbackInfo& info, std::vector<JsTypes> types)
{
    auto size = info.Length();
    if (size != types.size())
    {
        return false;
    }

#define IF_TYPE(TYPE) \
if (types[i] == JsTypes::TYPE)    \
{   \
    if (!info[i].Is##TYPE())    \
    {   \
        return false;   \
    }   \
}

    for (int i = 0; i < size; i++)
    {
        IF_TYPE(String)
            IF_TYPE(Number)
            IF_TYPE(Boolean)
            IF_TYPE(Object)
            IF_TYPE(Array)
            IF_TYPE(Buffer)
    }

    return true;
}

void throw_as_javascript_exception(Napi::Env& env, std::string message)
{
    Napi::TypeError::New(env, message).ThrowAsJavaScriptException();
}

class NapiTurnObserver : public TurnObserver
{
public:
    NapiTurnObserver(Napi::ObjectReference observer)
    {
        _observer.Reset(observer.Value());
    }

    ~NapiTurnObserver()
    {
        _observer.Unref();
    }

    void GetPassword(std::string& addr,
                     std::string& name,
                     std::function<void(std::optional<std::string>) > callback)
    {
        Napi::Env env = _observer.Env();
        Napi::Function func = _observer.Get("get_password").As<Napi::Function>();

        Napi::Value ret = func.Call({
            Napi::String::New(env, addr),
            Napi::String::New(env, name),
            Napi::Function::New(env, [&](const Napi::CallbackInfo& info)
            {
                if (!args_checker(info, { JsTypes::Boolean, JsTypes::String })) 
                {
                    throw_as_javascript_exception(env, "Wrong arguments");
                    return;
                }

                bool is_failed = info[0].As<Napi::Boolean>();
                if (is_failed)
                {
                    callback(std::nullopt);
                }
                else
                {
                    callback(info[0].As<Napi::String>().Utf8Value());
                }
            }) });
    }

private:
    Napi::ObjectReference _observer;
};

class NapiTurnProcesser : public Napi::ObjectWrap<NapiTurnProcesser>
{
public:
    static Napi::Object CreateInstance(Napi::Env env, TurnProcessor* processer)
    {
        Napi::Function func = DefineClass(env,
                                          "TurnProcesser",
                                          { InstanceMethod("process", &NapiTurnProcesser::Process) });
        Napi::FunctionReference* constructor = new Napi::FunctionReference();
        *constructor = Napi::Persistent(func);
        env.SetInstanceData(constructor);

        Napi::External<TurnProcessor> processer_ = Napi::External<TurnProcessor>::New(env, processer);
        return constructor->New({ processer_ });
    }

    NapiTurnProcesser(const Napi::CallbackInfo& info) : Napi::ObjectWrap<NapiTurnProcesser>(info)
    {
        Napi::Env env = info.Env();

        if (info.Length() != 1 || !info[0].IsExternal())
        {
            throw_as_javascript_exception(env, "Wrong arguments");
            return;
        }

        Napi::External<TurnProcessor> external = info[0].As<Napi::External<TurnProcessor>>();
        _processer = external.Data();
    }

    ~NapiTurnProcesser()
    {
        if (_processer != nullptr)
        {
            delete _processer;
        }
    }

    Napi::Value Process(const Napi::CallbackInfo& info)
    {
        Napi::Env env = info.Env();

        if (!args_checker(info, { JsTypes::Buffer, JsTypes::String, JsTypes::Function }))
        {
            throw_as_javascript_exception(env, "Wrong arguments");
            return env.Null();
        }

        Napi::Buffer<uint8_t> buffer = info[0].As<Napi::Buffer<uint8_t>>();
        std::string addr = info[1].As<Napi::String>().Utf8Value();
        Napi::FunctionReference func = Napi::Reference<Napi::Function>::New(info[2].As<Napi::Function>(), 1);

        _processer->Process(buffer.Data(),
                            buffer.Length(),
                            addr,
                            [&](bool is_success, ProcessResult* ret)
                            {
                                if (ret == nullptr)
                                {
                                    func.Call({ false, env.Null() });
                                }

                                if (is_success)
                                {
                                    Napi::Object response = Napi::Object::New(env);
                                    response.Set("data", Napi::Buffer<uint8_t>::NewOrCopy(env, ret->response.data, ret->response.data_len));
                                    response.Set("kind", Napi::String::New(env, ret->response.kind == StunClass::Msg ? "msg" : "channel"));
                                    response.Set("interface", Napi::String::New(env, ret->response.interface));
                                    response.Set("relay", Napi::String::New(env, ret->response.relay));
                                    func.Call({ false, response });
                                }
                                else
                                {
                                    func.Call({ true, Napi::Error::New(env, "").Value() });
                                }

                                func.Unref();
                            });
                            
        return env.Null();
    }

private:
    TurnProcessor* _processer = nullptr;
};

class NapiTurnService : public Napi::ObjectWrap<NapiTurnService>
{
public:
    static Napi::Object Init(Napi::Env env, Napi::Object exports)
    {
        Napi::Function func = DefineClass(env,
                                          "TurnService",
                                          { InstanceMethod("get_processer", &NapiTurnService::GetProcesser) });
        Napi::FunctionReference* constructor = new Napi::FunctionReference();
        *constructor = Napi::Persistent(func);
        env.SetInstanceData(constructor);
        exports.Set("TurnService", func);
        return exports;
    }

    NapiTurnService(const Napi::CallbackInfo& info) : Napi::ObjectWrap<NapiTurnService>(info)
    {
        Napi::Env env = info.Env();

        if (!args_checker(info, { JsTypes::String, JsTypes::Array, JsTypes::Object }))
        {
            throw_as_javascript_exception(env, "Wrong arguments");
            return;
        }

        std::string realm = info[0].As<Napi::String>().Utf8Value();
        Napi::Array externals = info[1].As<Napi::Array>();
        Napi::ObjectReference observer = Napi::ObjectReference::New(info[2].As<Napi::Object>(), 1);

        std::vector<std::string> externals_;
        for (size_t i = 0; i < externals.Length(); i++)
        {
            externals_.push_back(externals.Get(i).As<Napi::String>().Utf8Value());
        }

        try
        {
            _observer = std::make_unique<NapiTurnObserver>(std::move(observer));
            _servive = std::make_unique<TurnService>(realm, externals_, _observer.get());
        }
        catch (...)
        {
            throw_as_javascript_exception(env, "Failed to create turn service");
        }
    }

    Napi::Value GetProcesser(const Napi::CallbackInfo& info)
    {
        Napi::Env env = info.Env();

        if (!args_checker(info, { JsTypes::String, JsTypes::String }))
        {
            throw_as_javascript_exception(env, "Wrong arguments");
            return env.Null();
        }

        std::string interface = info[0].As<Napi::String>().Utf8Value();
        std::string external = info[1].As<Napi::String>().Utf8Value();
        TurnProcessor* processer = _servive->GetProcessor(interface, external);
        if (process == nullptr)
        {
            throw_as_javascript_exception(env, "Failed to get turn processer");
            return env.Null();
        }
        else
        {
            return NapiTurnProcesser::CreateInstance(info.Env(), processer);
        }
    }

private:
    std::unique_ptr<NapiTurnObserver> _observer;
    std::unique_ptr<TurnService> _servive;
};

Napi::Object Init(Napi::Env env, Napi::Object exports)
{
    return NapiTurnService::Init(env, exports);
}

NODE_API_MODULE(turn, Init);