import { Duplex } from 'stream'
import { Bytes } from 'bytes'

/**
 * codec
 * 
 * @module
 */

/**
 * Decoder
 * 
 * @class
 * @extends Duplex
 */
export class Decoder extends Duplex {
    private bytes: Bytes

    /**
     * @constructor
     */
    constructor() {
        super()
        this.bytes = new Bytes(4096)
    }

    /**
     * decode buffer.
     * 
     * @returns void
     * @private
     */
    private decode() {
        const size = this.bytes.raw.readUInt16BE(2) + 20
        this.bytes.length >= size && this.packet(size)
    }

    /**
     * push packet.
     * 
     * @param size packet size.
     * @returns void
     * @private
     */
    private packet(size: number) {
        const packet = this.bytes.slice(0, size)
        this.bytes.resize(0)
        this.push(packet)
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
        this.bytes.append(chunk)
        this.decode()
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
