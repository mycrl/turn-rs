const { Duplex } = require("stream")

/**
 * Mysticeti
 * 
 * 控制中心服务拓展
 * 主要为tarpc实现外部协议编解码器
 */
module.exports = class Mysticeti extends Duplex {
    constructor() {
        super()
        this.id = 0
        this.calls = {}
        this.listener = {}
        this.reader = Buffer.alloc(0)
    }
    
    /**
     * 绑定事件
     * @param {string} kind 事件类型
     * @param {function} handle 回调函数
     * @returns {void}
     * @public
     */
    bind(kind, handle) {
        this.listener[kind] = handle
    }
    
    /**
     * 呼叫事件
     * @param {string} kind 事件类型
     * @param {any} payload 消息
     * @returns {Promise<any>}
     * @public
     */
    call(kind, payload) {
        const id = this.id
    
        this.id += 1
        if (id >= 999999) {
            this.id = 0
        }
        
        const trace_context = { 
            parent_id: this._rand(), 
            trace_id: this._rand(), 
            span_id: this._rand()
        }
        
        const context = { 
            deadline: Date.now() + 10000, 
            trace_context
        }
        
        this._send({ Request: {
            message: { [kind]: payload },
            context,
            id
        }})
    
        return new Promise((resolve, reject) => {
            this.calls[id] = {
                resolve, 
                reject
            }
        })
    }
    
    /**
     * 随机数
     * @returns {number}
     * @private
     */
    _rand() {
        return Math.round(
            Math.random() * 999999
        )
    }
    
    /**
     * 发送消息到对端
     * @param {any} payload 消息
     * @returns {void}
     * @private
     */
    _send(payload) {
        let size_buf = Buffer.alloc(4)
        let buf = Buffer.from(JSON.stringify(payload))
        size_buf.writeUInt32BE(buf.length)
        this.push(Buffer.concat([
            size_buf,
            buf
        ]))
    }
  
    /**
     * 处理响应
     * @param {number} [request_id] 消息ID
     * @param {any} [message.Ok] 成功
     * @param {any} [message.Err] 错误
     * @returns {void}
     * @private
     */
    _handleReply({ request_id, message: { Ok, Err } }) {
        const handle = this.calls[request_id]
        if (!handle) return undefined
        if (Ok) handle.resolve(Ok)
        if (Err) handle.reject(Err)
        delete this.calls[request_id]
    }
  
    /**
     * 处理请求
     * @param {number} [Request.id] 消息ID
     * @param {any} [Request.message] 消息
     * @returns {Promise<void>}
     * @private
     */
    async _handleRequest({ Request: { message, id }}) {
        const kind = Object.keys(message)[0]
        const handle = this.listener[kind]
    
        if (!handle) {
            return undefined
        }
        
        let reply
        let type = "Ok"
    try {
        const res = handle(message[kind])
        reply = res instanceof Promise ? await res : res
    } catch (err) {
        reply = err
        type = "Err"
    }
        
        this._send({
            message: { [type]: { [kind]: reply }},
            request_id: id,
        })
    }
  
    /**
     * 双工流实现
     * @param {Buffer} chunk 缓冲区分片
     * @param {function} callback 写入回调
     * @returns {void}
     * @private
     */
    _read(_size) {}
    _write(chunk, _encoding, callback) {
        this.reader = Buffer.concat([
            this.reader,
            chunk
        ])
        
        let count = 0
    for (;;) {
        if (this.reader.length - count <= 8) {
            break
        }

        const size = this.reader.readUInt32BE(count)
        const buf = this.reader.subarray(count + 4, count + 4 + size)
        const payload = JSON.parse(buf.toString())
        
        count += size + 4

        if ("Request" in payload) {
            this._handleRequest(payload)
        }

        if ("message" in payload && "request_id" in payload) {
            this._handleReply(payload)
        }
    }
        
        this.reader = this.reader
            .slice(count)
        callback()
    }
}