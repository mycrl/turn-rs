import { Duplex } from "stream"

// 触发器服务
export enum Trigger {
    Auth = 0
}

// 状态服务
export enum Service {
    Get = 1,
    Remove = 2
}

// 请求
export interface Request {
    addr: string
}

// 认证请求
//
// * `addr` 客户端地址
// * `username` 用户名
export interface AuthRequest {
    addr: string
    username: string
}

// 认证信息
//
// * `password` 密钥
// * `group` 分组ID
export interface Auth {
    password: string
    group: number
}

// 节点
//
// * `group` 分组ID
// * `delay` 超时时间
// * `clock` 内部时钟
// * `password` 密钥
// * `ports` 分配端口列表
// * `channels` 分配频道列表
export interface Node {
    group: number
    delay: number
    clock: number
    ports: number[]
    channels: number[]
    password: string
}

// 负载类型
//
// * `Request` 请求
// * `Reply` 正确响应
// * `Error` 错误响应
enum Flag {
    Request = 0,
    Reply = 1,
    Error = 2
}

// 呼叫反射
interface ReflectCall {
    [Service.Get]: Node
    [Service.Remove]: null
}

// 消息负载
//
// * `data` 消息内容
// * `kind` 消息属性
// * `flag` 消息类型
// * `id` 消息ID
export interface Payload {
    data: Buffer
    kind: number
    flag: Flag
    id: number
}

// 处理程序
//
// 闭包或者异步闭包类型
export type Handler<T, U> = (d: T) => U | Promise<U>

// Future
// 异步回调栈类型
export interface Future<T> {
    success: (d: T) => void
    fail: (e: Error) => void
    clock: number
}

// 配置
//
// * `timeout` 消息超时
export interface MysticetiOptions {
    timeout: number
}

// 默认配置
export const DefaultConf: MysticetiOptions = {
    timeout: 10000
}

// Mysticeti
// @desc 双工流，对流添加RPC支持
// @class
export default class Mysticeti extends Duplex {
    private listener: { [key: number]: Handler<any, any> }
    private futures: { [key: number]: Future<any> }
    private buffer: Buffer
    private id: number
    
    // @param config 配置
    // @constructor
    constructor(
        private config?: MysticetiOptions
    ) {
        super()
        this.id = 0
        this.futures = {}
        this.listener = {}
        this.buffer = Buffer.alloc(0)
        setInterval(this.poll.bind(this), 5000)
    }
    
    // 处理正确响应
    //
    // 处理远程响应消息，并唤醒Promise回调结果
    private handle_reply({ id, data }: Payload) {
        const stack = this.futures[id]
        if (!stack) return undefined
        stack.success(JSON.parse(data.toString()))
        delete this.futures[id]
    }
    
    // 处理错误响应
    //
    // 处理远程响应消息，并唤醒Promise回调错误
    private handle_error({ id, data }: Payload) {
        const stack = this.futures[id]
        if (!stack) return undefined
        stack.fail(new Error(data.toString()))
        delete this.futures[id]
    }
    
    // 处理请求
    //
    // 处理远端请求消息，唤醒实例处理程序
    // 并将结果返回远端
    private async handle_request(payload: Payload) {
        const handler = this.listener[payload.kind]
        if (!handler) return undefined
 
        let data
        let flag = Flag.Reply
    try {
        const res = handler(JSON.parse(payload.data.toString()))
        const replay = res instanceof Promise ? await res : res
        data = Buffer.from(JSON.stringify(replay))
    } catch(e) {
        data = Buffer.from(e.message)
        flag = Flag.Error
    }

        this.send({
            kind: payload.kind,
            id: payload.id,
            flag,
            data
        })
    }
    
    // 发送消息到远端
    //
    // 直接推送到流水线的下个位置
    // 主动推送，避免流无法被唤醒
    private send({ kind, flag, id, data }: Payload) {
        let buf = Buffer.alloc(10)
        
        buf[4] = kind
        buf[5] = flag
        buf.writeUInt32BE(data.length, 0)
        buf.writeUInt32BE(id, 6)
        
        this.push(Buffer.concat([
            buf,
            data
        ]))
    }
    
    // 读取流数据
    //
    // 目前流水线存在唤醒延迟的情况，
    // 会造成消息无法及时发送到远端的情况，
    // 所以这里不处理流水线的主动唤醒
    _read(_size: number) {}
    
    // 写入流数据
    //
    // 流水线推送缓冲区分片，将缓冲区分片
    // 推送到内部缓冲区暂存，并尽量解码出消息，
    // 直到无法继续处理，收缩内部缓冲区
    _write(chunk: Buffer, _encoding: string, callback: () => void) {
        this.buffer = Buffer.concat([
            this.buffer,
            chunk
        ])
        
        let offset = 0
    for(;;) {
        if (this.buffer.length - offset <= 10) {
            break;
        }

        const size = this.buffer.readUInt32BE(offset)
        const cursor = offset + size + 10
        if (cursor < this.buffer.length) {
            break
        }

        const kind = this.buffer[offset + 4]
        const flag = this.buffer[offset + 5]
        const id = this.buffer.readUInt32BE(offset + 6)
        const data = this.buffer.subarray(offset + 10, cursor)
        const payload = {
            kind, flag,
            id, data
        }
        
        offset = cursor
        if (flag === Flag.Request) this.handle_request(payload)
        if (flag === Flag.Reply) this.handle_reply(payload)
        if (flag === Flag.Error) this.handle_error(payload)
    }
        
        if (offset > 0) {
            this.buffer = this.buffer.slice(offset)
        }
        
        callback()
    }
    
    // 内部循环
    //
    // 定时清理失效Future，以免造成内存溢出
    // 这是必要的操作，因为并不能保证远端确定回复
    private poll() {
        const now = Date.now()
        const { 
            timeout = DefaultConf.timeout 
        } = this.config || DefaultConf
        for (const id in this.futures) {
            if (now - this.futures[id].clock >= timeout) {
                this.futures[id].fail(new Error("timeout"))
                delete this.futures[id]
            }
        }
    }
    
    // 创建Future
    //
    // 将Future推入内部列表
    // Future将和消息ID绑定
    private future<T>(id: number): Promise<T> {
        return new Promise((success, fail) => {
            this.futures[id] = {
                clock: Date.now(),
                success, 
                fail
            }
        })
    }
    
    // 绑定事件
    //
    // 当RPC有当前事件请求时，触发处理程序
    // 处理程序可以为async fn，内部将自动处理差异
    //
    // @param kind 事件
    // @param handler 处理程序
    public bind(kind: Trigger, handler: Handler<AuthRequest, Auth>) {
        this.listener[kind] = handler
    }
    
    // 呼叫远程
    //
    // 呼叫远程对应事件监听器，当得到回应时，
    // Promise将被Ready，返回远端消息
    //
    // @param kind 事件
    // @param message 请求消息
    public call<
        T extends keyof ReflectCall,
    >(kind: T, message: Request): Promise<ReflectCall[T]> {
        this.id = this.id >= 4294967295 ? 0 : this.id + 1

        const id = this.id
        const flag = Flag.Request
        const data = Buffer.from(JSON.stringify(message))
        this.send({ flag, id, kind, data })
        return this.future(id) 
    }
}
