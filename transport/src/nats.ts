import { connect, Stan } from "node-nats-streaming"
import { Payload } from "./yamux"
import { Writable } from "stream"

export class Nats extends Writable {
    private stan: Stan

    constructor (cluster: string, node: string, server: any) {
        super({ objectMode: true })
        this.stan = connect(cluster, node, server)
    }
    
    // 写入
    // @param {chunk} 消息
    // @param {callback} 回调
    public _write (chunk: Payload, _: string, callback: any): void {
        this.stan.publish(chunk.uid, chunk.packet.data, (err, guid) => {
            console.log(guid)
            callback(err ? err : null)
        })
    }
    
    // 完成
    // @param {callback} 回调
    public _final (callback: any): void {
        callback(null)
    }
}
