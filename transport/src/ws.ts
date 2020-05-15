import { Writable } from "stream"
import WebSocket from "ws"
import http from "http"

export class Ws extends Writable {
    private server: http.Server
    private ws: WebSocket.Server
    private socket?: WebSocket

    constructor (port: number) {
        super()
        this.server = http.createServer()
        this.ws = new WebSocket.Server({ server: this.server })
        this.ws.on("connection", this.connection.bind(this))
        this.server.listen(port)
    }

    private connection (socket: WebSocket) {
        this.socket = socket
    }

    // 写入
    // @param {chunk} 消息
    // @param {callback} 回调
    public _write (chunk: Buffer, _: string, callback: any): void {
        this.socket?.send(chunk)
        callback(null)
    }
    
    // 完成
    // @param {callback} 回调
    public _final (callback: any): void {
        callback(null)
    }
}
