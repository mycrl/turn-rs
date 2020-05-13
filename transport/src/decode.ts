import { Duplex } from "stream"
import { RemoteInfo } from "dgram"
import { Message } from "./dgram"
import { v4 } from "uuid"

export enum Media {
    Video = 0,
    Audio = 1
}

export interface Payload {
    uid: string
    data: Buffer
}

export interface Packet {
    flg: Media,
    mid: number
    len: number
    pid: number
    end: boolean
    data: Buffer
}

export interface Info {
    address: string
    family: string
    port: number
}

type Values<T> = { [id: number]: T }
type MsgItem = { update: number, end: boolean, values: Values<Buffer> }
type SesssionItem = { update: number, values: Values<MsgItem> }
type Buffers = { [uid: string]: SesssionItem }

export class Decode extends Duplex {
    private uid: Map<Info, string>
    private buffer: Buffers

    constructor () {
        super({ objectMode: true })
        this.uid = new Map()
        this.buffer = {}
    }

    // 解码数据包
    // @param {message} 数据包
    private decode (message: Buffer): Packet {
        let flg = message[0]
        let mid = message.readIntBE(1, 4)
        let len = message.readIntBE(5, 4)
        let pid = message[9]
        let end = message[10] == 1
        let data = message.slice(11, len)
        return { flg, mid, len, pid, end, data }
    }

    // 获取会话信息
    // 剔除消息长度变量
    // @param {session} 远程会话信息
    private sign (session: RemoteInfo): Info {
        delete session.size
        return session
    }

    // 获取会话ID
    // @param {session} 会话信息
    private forwrad (session: Info): string {
        if (!this.uid.has(session))
            this.uid.set(session, v4())
        return <string>this.uid.get(session)
    }

    // 聚合数据
    // TODO: 未测试可靠性
    private join (uid: string) {
        for (let uid in this.buffer) {
            let mids = Object.keys(this.buffer[uid].values)
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

    // 处理会话数据
    // @param {message} 会话数据
    private process ({ message, session }: Message) {
        let uid = this.forwrad(this.sign(session))
        let packet = this.decode(message)
        let date = Date.now()

        // 检查消息列表是否初始化
        if (!this.buffer[uid]) {
            this.buffer[uid] = {
                update: 0,
                values: {}
            }
        }

        // 检查包列表是否初始化
        if (!this.buffer[uid].values[packet.mid]) {
            this.buffer[uid].values[packet.mid] = {
                end: false,
                update: 0,
                values: {}
            }
        }

        // 写入包数据
        // 聚合数据
        this.buffer[uid]
            .update = date
        this.buffer[uid]
            .values[packet.mid]
            .update = date
        this.buffer[uid]
            .values[packet.mid]
            .end = packet.end
        this.buffer[uid]
            .values[packet.mid]
            .values[packet.pid] = packet.data
        this.join(uid)
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
