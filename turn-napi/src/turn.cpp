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
};

bool args_checker(const Napi::CallbackInfo& info, std::vector<JsTypes> types)
{
    auto size = info.Length();
    if (size != types.size())
    {
        return false;
    }

    for (int i = 0; i < size; i++)
    {
        if (types[i] == JsTypes::String)
        {
            if (!info[i].IsString())
            {
                return false;
            }
        }
        else if (types[i] == JsTypes::Number)
        {
            if (!info[i].IsNumber())
            {
                return false;
            }
        }
        else if (types[i] == JsTypes::Boolean)
        {
            if (!info[i].IsBoolean())
            {
                return false;
            }
        }
        else if (types[i] == JsTypes::Object)
        {
            if (!info[i].IsObject())
            {
                return false;
            }
        }
        else if (types[i] == JsTypes::Array)
        {
            if (!info[i].IsArray())
            {
                return false;
            }
        }
    }

    return true;
}

class NapiTurnObserver : public TurnObserver
{
public:
    NapiTurnObserver(Napi::Object observer) : _observer(observer)
    {
    }

    void GetPassword(std::string& addr,
                     std::string& name,
                     std::function<void(std::optional<std::string>) > callback)
    {
        Napi::Env env = _observer.Env();
        Napi::Function func = _observer.Get("get_password").As<Napi::Function>();
        Napi::Value ret = func.Call({ Napi::String::New(env, addr), Napi::String::New(env, name) });
    }

private:
    Napi::Object _observer;
};

class NapiTurnProcesser
{
public:
    static Napi::Object Init(Napi::Env env, Napi::Object exports)
    {
        typedef Napi::ObjectWrap<NapiTurnProcesser> Wrap;
        Napi::Function func = Wrap::DefineClass(env, "TurnProcesser", { Wrap::InstanceMethod("process", &NapiTurnProcesser::Process) });

    }

    NapiTurnProcesser(std::unique_ptr<TurnProcessor> processer) : _processer(processer)
    {
    }

    Napi::Value Process(const Napi::CallbackInfo& info)
    {

    }

private:
    std::unique_ptr<TurnProcessor> _processer;
};

class NapiTurnService : public Napi::ObjectWrap<NapiTurnService>
{
public:
    static Napi::Object Init(Napi::Env env, Napi::Object exports)
    {
        Napi::Function func = DefineClass(env, "TurnService", { InstanceMethod("get_processer", &NapiTurnService::GetProcesser) });
        Napi::FunctionReference* constructor = new Napi::FunctionReference();
        *constructor = Napi::Persistent(func);
        env.SetInstanceData(constructor);
        exports.Set("TurnService", func);
        return exports;
    }

    NapiTurnService(const Napi::CallbackInfo& info) : Napi::ObjectWrap<NapiTurnService>(info)
    {
        args_checker(info, { JsTypes::String, JsTypes::Array, JsTypes::Object });

        Napi::Env env = info.Env();
        std::string realm = info[0].As<Napi::String>().Utf8Value();
        Napi::Array externals = info[1].As<Napi::Array>();
        Napi::Object observer = info[2].As<Napi::Object>();

        std::vector<std::string> externals_;
        for (int i = 0; i < externals.Length(); i++)
        {
            externals_.push_back(externals.Get(i).As<Napi::String>().Utf8Value());
        }

        _observer = std::make_unique<NapiTurnObserver>(observer);
        _servive = std::make_unique<TurnService>(realm, externals_, _observer.get());
    }

    Napi::Value GetProcesser(const Napi::CallbackInfo& info)
    {
        args_checker(info, { JsTypes::String, JsTypes::String });

        std::string interface = info[0].As<Napi::String>().Utf8Value();
        std::string external = info[1].As<Napi::String>().Utf8Value();
        TurnProcessor processer = _servive->GetProcessor(interface, external);
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