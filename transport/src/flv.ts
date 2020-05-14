import { Duplex } from "stream"
import { Payload, Flg } from "./yamux"

// 媒体头类型
export enum HeaderType {
    Audio = 0x04,
    Video = 0x01,
    AudioAndVideo = 0x05
}

// 媒体类型
export enum MediaType {
    MetaData = 0x12,
    Audio = 0x08,
    Video = 0x09
}

// 创建FLV数据
// @param {data} 音视频数据
// @param {type} 媒体类型
// @param {timestamp} 时间戳
export function createFLV (data: Buffer, type: MediaType, timestamp = 0): Buffer {
    let size = data.length + 11
    let tag = Buffer.alloc(size + 4)
    tag[0] = type
    tag.writeUIntBE(data.length, 1, 3)
    tag[4] = (timestamp >> 16) & 0xff
    tag[5] = (timestamp >> 8) & 0xff
    tag[6] = timestamp & 0xff
    tag[7] = (timestamp >> 24) & 0xff
    tag.writeUIntBE(0, 8, 3)
    data.copy(tag, 11)
    tag.writeUInt32BE(size, size)
    return tag
}

// 创建FLV头
// @param {type} 媒体头类型
export function createHeader (type: HeaderType): Buffer {
    return Buffer.from([

        // "FLV"
        0x46, 
        0x4c,
        0x56, 

        // version
        0x01,

        // flgs
        type,

        // size
        0x00, 
        0x00, 
        0x00, 
        0x09,

        // size
        0, 0, 0, 0
    ])
}

// 创建媒体信息
// @param {data} 媒体数据
export function createMetaData (data: Buffer) {
    return createFLV(data, MediaType.MetaData)
}

// 创建音频信息
// @param {data} 音频数据
export function createAudio (data: Buffer) {
    return createFLV(data, MediaType.Audio)
}

// 创建视频信息
// @param {data} 视频数据
export function createVideo (data: Buffer) {
    return createFLV(data, MediaType.Video)
}

// Flv
// @class
export class Flv extends Duplex {
    constructor () {
        super({ objectMode: true })
    }
    
    // 读取
    // @param {size} 长度
    public _read (size: number): void {
        this.read(size)
    }
    
    // 写入
    // @param {chunk} 消息
    // @param {callback} 回调
    public _write (chunk: Payload, _: string, callback: any): void {
        if (chunk.packet.flg === Flg.Frame) {
            this.push(createHeader(HeaderType.AudioAndVideo))
            this.push(createMetaData(chunk.packet.data))
        } else
        if (chunk.packet.flg === Flg.Audio) {
            this.push(createAudio(chunk.packet.data))
        } else
        if (chunk.packet.flg === Flg.Video) {
            this.push(createVideo(chunk.packet.data))
        }

        callback(null)
    }
    
    // 完成
    // @param {callback} 回调
    public _final (callback: any): void {
        callback(null)
    }
}
