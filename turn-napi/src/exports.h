//
//  turn-napi.h
//  turn-napi
//
//  Created by Mr.Panda on 2023/12/16.
//

#ifndef TURN_NAPI_H
#define TURN_NAPI_H
#pragma once

#include <memory>
#include <napi.h>

#include "turn.h"

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

bool args_checker(const Napi::CallbackInfo& info, std::vector<JsTypes> types);
void throw_as_javascript_exception(Napi::Env& env, std::string message);
void run_promise(Napi::Function& async_func,
                 const std::vector<Napi::Value>& args,
                 std::function<void(const Napi::Value&)> resolve,
                 std::function<void(const Napi::Error&)> reject);

class NapiTurnObserver : public TurnObserver
{
public:
    NapiTurnObserver(Napi::ObjectReference observer);
    ~NapiTurnObserver();

    void GetPassword(std::string& addr,
                     std::string& name,
                     std::function<void(std::optional<std::string>)> callback) override;
private:
    Napi::ObjectReference _observer;
};

class NapiTurnProcesser : public Napi::ObjectWrap<NapiTurnProcesser>
{
public:
    class ProcessAsyncWorker : public Napi::AsyncWorker
    {
    public:
        ProcessAsyncWorker(const Napi::Env& env,
                           TurnProcessor* processer,
                           std::string addr,
                           uint8_t* buf,
                           size_t buf_size);

        void Execute() override;
        void OnOK() override;
        void OnError(const Napi::Error& err) override;
        Napi::Promise GetPromise();
    private:
        std::shared_ptr<TurnProcessor::Results> _result = nullptr;
        TurnProcessor* _processer = nullptr;
        Napi::Promise::Deferred _deferred;
        std::string _addr;
        uint8_t* _buf;
        size_t _buf_size;
    };

    NapiTurnProcesser(const Napi::CallbackInfo& info);
    ~NapiTurnProcesser();

    static Napi::Object CreateInstance(Napi::Env env, TurnProcessor* processer);
    Napi::Value Process(const Napi::CallbackInfo& info);
private:
    TurnProcessor* _processer = nullptr;
};

class NapiTurnService : public Napi::ObjectWrap<NapiTurnService>
{
public:
    NapiTurnService(const Napi::CallbackInfo& info);

    static Napi::Object Init(Napi::Env env, Napi::Object exports);
    Napi::Value GetProcesser(const Napi::CallbackInfo& info);
private:
    std::unique_ptr<NapiTurnObserver> _observer;
    std::unique_ptr<TurnService> _servive;
};

#endif // TURN_NAPI_H
