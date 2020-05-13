import dgram, { Socket, RemoteInfo } from "dgram"
import { Readable } from "stream"

export interface Message {
    message: Buffer
    session: RemoteInfo
}

export class Dgram extends Readable {
    private dgram: Socket

    constructor (port: number) {
        super({ objectMode: true })
        this.dgram = dgram.createSocket("udp4")
        this.initialize(port)
    }

    private initialize (port: number): void {
        this.dgram.on("message", this.message.bind(this))
        this.dgram.bind(port)
    }

    private message (message: Buffer, session: RemoteInfo): void {
        this.push({ message, session })
    }

    // 读取
    // @param {size} 长度
    public _read (size: number): void {
        this.read(size)
    }
}
