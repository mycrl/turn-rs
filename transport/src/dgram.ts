import dgram, { Socket, RemoteInfo } from "dgram"
import { Readable } from "stream"

// 流消息类型
// @param {message} 数据
// @param {session} 消息信息
export interface Message {
    message: Buffer
    session: RemoteInfo
}

// Udp类
// @class
export class Dgram extends Readable {
    private dgram: Socket

    // @constructor
    // @param {port} 监听端口
    constructor (port: number) {
        super({ objectMode: true })
        this.dgram = dgram.createSocket("udp4")
        this.initialize(port)
    }

    // 初始化
    // @param {port} 监听端口
    private initialize (port: number): void {
        this.dgram.on("message", this.message.bind(this))
        this.dgram.bind(port)
    }

    // Udp消息处理
    // @param {message} 数据
    // @param {session} 消息信息
    private message (message: Buffer, session: RemoteInfo): void {
        this.push(<Message>{ message, session })
    }

    // 读取
    // @param {size} 长度
    public _read (size: number): void {
        this.read(size)
    }
}
