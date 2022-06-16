import { Socket, createSocket, SocketOptions } from 'dgram'
import { Config } from './configure'
import { Duplex } from 'stream'

/**
 * proxy
 * 
 * @module
 */

/**
 * default socket opions.
 * 
 * @readonly
 */
const DefaultSocketOptions: SocketOptions = {
    type: 'udp4',
    ipv6Only: false,
    recvBufferSize: 2048,
    sendBufferSize: 2048,
}

/**
 * create udp socket.
 * 
 * @returns udp socket.
 * @private 
 */
function createUdpSocket() {
    const socket = createSocket(DefaultSocketOptions)
    socket.bind(0)
    return socket
}

/**
 * Proxy
 * 
 * @class
 * @extends Duplex
 */
export class Proxy extends Duplex {
    private socket: Socket
    
    /**
     * @constructor
     */
    constructor() {
        super()
        this.socket = createUdpSocket()
        this.socket.on('message', this.onMsg.bind(this))
    }

    /**
     * handle decode message.
     * 
     * @param tcp socket packet message.
     * @returns void
     * @private
     */
    private onMsg(msg: Buffer) {
        this.push(msg)
    }

    /**
     * stream write.
     * 
     * @param chunk tcp buffer chunk.
     * @param encoding chunk encodeing.
     * @param next callback.
     * @private
     */
     _write(chunk: Buffer, _: string, next: any) {
        this.socket.send(chunk, Config.proxy_port, Config.proxy_ip)
        next()
    }

    /**
     * stream read.
     * 
     * @param size read size. 
     * @private
     */
    _read(_size: number) {
        
    }
}
