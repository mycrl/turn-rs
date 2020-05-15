import { Duplex } from "stream"

export interface Packet {
    flag: number,
    body: Buffer
}

// 编解码
// @class
export class Codec extends Duplex {
    private buffer: Buffer

    constructor () {
        super({ objectMode: true })
        this.buffer = Buffer.alloc(0)
    }

    private decoder (chunk: Buffer) {
        this.buffer = Buffer.concat([ 
            this.buffer,
            chunk 
        ])

        for (;;) {
            let head = this.buffer.readUInt32BE(0)
            if (head != 0x99999909) {
                break
            }

            let flag = this.buffer[4]
            let size = this.buffer.readUInt32BE(5)
            let last = this.buffer.length - 9
            if (last < size) {
                break
            }

            let body = this.buffer.slice(9, 9 + size)
            this.buffer = this.buffer.slice(size + 9)
            this.push(<Packet>{ flag, body })

            if (this.buffer.length < 10) {
                break
            }
        }
    }

    private encoder (packet: Packet) {
        let buffer = Buffer.alloc(9)
        let size = packet.body.length
        
        buffer.writeUInt32BE(0x99999909)
        buffer.writeUInt8(packet.flag, 4)
        buffer.writeUInt32BE(size, 5)

        this.push(Buffer.concat([
            buffer,
            packet.body
        ]))
    }

    public _read (size: number) {
        this.read(size)
    }
    
    public _write (chunk: Packet | Buffer, _: string, callback: any) {
        Buffer.isBuffer(chunk) ?
            this.decoder(chunk) :
            this.encoder(chunk)
        callback(null)
    }
    
    public _final (callback: any) {
        callback(null)
    }
}
