import net, { createServer } from "net"
import { pipeline } from "stream"
import { Codec } from "./lib/codec"
import { Flv } from "./lib/flv"
import { Ws } from "./ws"

// Server类
// @class
export class Server {
    private server: net.Server

    // @constructor
    // @param {port} 监听端口
    constructor (port: number) {
        this.server = createServer()
        this.initialize(port)
    }

    // 初始化
    // @param {port} 监听端口
    private initialize (port: number): void {
        this.server.on("connection", this.connection.bind(this))
        this.server.listen(port)
    }

    // 新连接处理
    // @param {message} 数据
    // @param {session} 消息信息
    private connection (socket: net.Socket): void {
        pipeline(
            socket,
            new Codec(),
            new Flv(),
            new Ws(80),
            (err) => console.error(err)
        )
    }
}
