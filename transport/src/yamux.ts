import { Duplex } from "stream"
import { Message } from "./dgram"
import { v4 } from "uuid"

// @type
type Values<T> = { [id: number]: T }
type MsgItem = { update: number, end: boolean, values: Values<Buffer> }
type SesssionItem = { update: number, values: Values<MsgItem> }
type Transfer = { [uid: string]: SesssionItem }

// 标志位类型
export enum Flag {
    Video = 0x00,
    Audio = 0x01,
    Frame = 0x02,
    Publish = 0x03,
    UnPublish = 0x04
}

// 有效载荷类型
// @param {uid} 会话ID
// @param {packet} 消息信息
export interface Payload {
    uid: string
    packet: Packet
}

// 消息包类型
// @param {flag} 标志位
// @param {mid} 消息ID
// @param {len} 消息长度
// @param {pid} 包ID
// @param {end} 包是否结束
// @param {data} 消息数据
export interface Packet {
    flag: Flag
    mid: number
    len: number
    pid: number
    end: boolean
    data: Buffer
}

// 会话信息类型
// @param {address} 地址
// @param {family} 协议
// @param {port} 端口
export interface Info {
    address: string
    family: string
    port: number
}

// 多路解复用
// @class
export class Yamux  extends Duplex {
    private uid: Map<Info, string>
    private transfer: Transfer

    // @constructor
    constructor () {
        super({ objectMode: true })
        this.uid = new Map()
        this.transfer = {}
    }

    // 解码数据包
    // @param {message} 数据包
    private decode (message: Buffer): Packet {
        let flag = message[0]
        let mid = message.readIntBE(1, 4)
        let len = message.readIntBE(5, 4)
        let pid = message[9]
        let end = message[10] == 1
        let data = message.slice(11, len)
        return <Packet>{ 
            flag, mid, len, 
            pid, end, data 
        }
    }

    // 获取会话信息
    // 剔除消息长度变量
    // @param {session} 远程会话信息
    private sign (session: Message["session"]): Info {
        delete session.size
        return session
    }

    // 获取会话ID
    // @param {session} 会话信息
    private forward (session: Info): string {
        if (!this.uid.has(session))
            this.uid.set(session, v4())
        return <string>this.uid.get(session)
    }

    // 跟踪数据
    // @param {uid} 会话ID
    private track (uid: string) {
        for (let uid in this.transfer) {
            let mids = Object.keys(this.transfer[uid].values)
                .map(Number)
                .sort((x, y) => x - y)
            for (let mid of mids) {
                let { end, values } = this
                    .buffer[uid].values[mid]
                Object.keys(values)
                    .map(Number)
                    .sort((x, y) => x - y)
                    .map(x => values[x])
                    .map(data => {
                        data.length > 0 && this.push({ uid, data })
                        // TODO: 未删除缓存
                    })
                if (end) {
                    break
                }
            }
        }
    }

    // 处理Udp数据包
    // @param [message] 数据
    // @param [session] 会话信息
    private process ({ message, session }: Message) {
        let uid = this.forward(this.sign(session))
        let packet = this.decode(message)
        let date = Date.now()

        if (!this.transfer[uid]) {
            this.transfer[uid] = {
                update: 0,
                values: {}
            }
        }

        if (!this.transfer[uid].values[packet.mid]) {
            this.transfer[uid].values[packet.mid] = {
                end: false,
                update: 0,
                values: {}
            }
        }

        this.transfer[uid]
            .update = date
        this.transfer[uid]
            .values[packet.mid]
            .update = date
        this.transfer[uid]
            .values[packet.mid]
            .end = packet.end
        this.transfer[uid]
            .values[packet.mid]
            .values[packet.pid] = packet.data
        
        this.track(uid)
    }

    // 读取
    // @param {size} 长度
    public _read (size: number): void {
        this.read(size)
    }
    
    // 写入
    // @param {chunk} 消息
    // @param {callback} 回调
    public _write (chunk: Message, _: string, callback: any): void {
        this.process(chunk)
        callback(null)
    }
    
    // 完成
    // @param {callback} 回调
    public _final (callback: any): void {
        callback(null)
    }
}
