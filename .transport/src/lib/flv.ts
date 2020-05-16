import { Duplex } from "stream"
import { Packet } from "./codec"

const FLAG_VIDEO = 0
const FLAG_AUDIO = 1
const FLAG_FRAME = 2
const FLAG_PUBLISH = 3
const FLAG_UNPUBLISH = 4

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

    // 创建FLV数据
    // @param {data} 音视频数据
    // @param {type} 媒体类型
    // @param {timestamp} 时间戳
    private createFLV (data: Buffer, type: MediaType, timestamp = 0): Buffer {
        let size = data.length + 11
        let tag = Buffer.alloc(size + 4)
        tag[0] = type
        tag.writeUIntBE(data.length, 1, 3)
        tag[4] = (timestamp >> 16) & 0xff
        tag[5] = (timestamp >> 8) & 0xff
        tag[6] = timestamp & 0xff
        tag[7] = (timestamp >> 24) & 0xff
        tag.writeUIntBE(0, 8, 3)
        tag.writeUInt32BE(size, size)
        data.copy(tag, 11)
        return tag
    }

    // 创建FLV头
    // @param {type} 媒体头类型
    private createHeader (type: HeaderType): Buffer {
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
    private createMetaData (data: Buffer, timestamp = 0) {
        return this.createFLV(data, MediaType.MetaData, timestamp)
    }

    // 创建音频信息
    // @param {data} 音频数据
    private createAudio (data: Buffer, timestamp = 0) {
        return this.createFLV(data, MediaType.Audio, timestamp)
    }

    // 创建视频信息
    // @param {data} 视频数据
    private createVideo (data: Buffer, timestamp = 0) {
        return this.createFLV(data, MediaType.Video, timestamp)
    }
    
    // 写入
    // @param {chunk} 消息
    // @param {callback} 回调
    public _write (chunk: Packet, _: string, callback: any) {
        if (chunk.flag === FLAG_FRAME) {
            this.push(this.createHeader(HeaderType.AudioAndVideo))
            this.push(this.createMetaData(chunk.body))
        } else
        if (chunk.flag === FLAG_AUDIO) {
            let timestamp = chunk.body.readUInt32BE(0)
            this.push(this.createAudio(chunk.body.slice(4), timestamp))
        } else
        if (chunk.flag === FLAG_VIDEO) {
            let timestamp = chunk.body.readUInt32BE(0)
            this.push(this.createVideo(chunk.body.slice(4), timestamp))
        }

        callback(null)
    }
    
    // 完成
    // @param {callback} 回调
    public _final (callback: any) {
        callback(null)
    }
}
