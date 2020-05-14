import { pipeline, Writable } from "stream"
import { Dgram } from "./dgram"
import { Yamux } from "./yamux"
// import { Nats } from "./nats"
import { Flv } from "./flv"

import WebSocket from "ws"
import http from "http"

class Ws extends Writable {
    private server: http.Server
    private ws: WebSocket.Server
    private socket?: WebSocket

    constructor () {
        super()
        this.server = http.createServer()
        this.ws = new WebSocket.Server({ server: this.server })
        this.ws.on("connection", this.connection.bind(this))
        this.server.listen(80)
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

pipeline(
    new Dgram(1936),
    new Yamux(),
    new Flv(),
    // new Nats("test-cluster", "test", "nats://localhost:4222"),
    new Ws(),
    (err) => {
    console.error(err)
})
