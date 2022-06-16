'use strict'

/** 
 * @module storage/base/Bytes
 */

/**
 * Heap allocation buffer.
 * @class Bytes
 * @example
 * import { deepStrictEqual } from "assert"
 * 
 * const bytes = new Bytes(100)
 * 
 * deepStrictEqual(bytes.push(0), true)
 * deepStrictEqual(bytes.push(1), true)
 * deepStrictEqual(bytes.length, 2)
 * deepStrictEqual(bytes.slice().length, 2)
 * deepStrictEqual(bytes.slice(0, 1).length, 1)
 * 
 * bytes.resize(1)
 * deepStrictEqual(bytes.length, 1)
 * deepStrictEqual(bytes.capacity, 100)
 */
exports.Bytes = class Bytes {
    
    /**
     * @param {number} size - bytes capacity.
     * @constructor
     */
    constructor(size = 0) {
        this._raw = Buffer.allocUnsafe(size)
        this._max = size - 1
        this._len = 0
    }
    
    /**
     * Expend raw buffer capacity.
     * @param {number?} size
     * @returns {void}
     * @private
     */
    _expand(size = 1024) {
        const len = size > 1024 ? Math.ceil(size / 1024) * 1024 : 1024
        this._raw = Buffer.concat([ this._raw, Buffer.allocUnsafe(len) ])
        this._max += len
    }
    
    /**
     * Appends an element to the back of a collection.
     * @param {number} byte
     * @returns {boolean}
     * @public
     * @example
     * import { deepStrictEqual } from "assert"
     * 
     * const bytes = new Bytes(100)
     * 
     * deepStrictEqual(bytes.push(0), true)
     * deepStrictEqual(bytes.push(1), true)
     */
    push(byte) {
        if (this._len === this._max) this._expand()
        this._raw[this._len] = byte
        this._len += 1
        return true
    }
    
    /**
     * get raw buffer in self.
     * @returns {Buffer}
     * @public
     */
     get raw() {
        return this._raw
    }
    
    /**
     * Returns the number of elements in the vector, 
     * also referred to as its ‘length’.
     * @returns {number}
     * @public
     * @example
     * import { deepStrictEqual } from "assert"
     * 
     * const bytes = new Bytes(100)
     * 
     * deepStrictEqual(bytes.push(0), true)
     * deepStrictEqual(bytes.push(1), true)
     * deepStrictEqual(bytes.length, 2)
     */
    get length() {
        return this._len
    }
    
    /**
     * Get raw buffer capacity.
     * @returns {number}
     * @public
     * @example
     * import { deepStrictEqual } from "assert"
     * 
     * const bytes = new Bytes(100)
     * 
     * deepStrictEqual(bytes.push(0), true)
     * deepStrictEqual(bytes.push(1), true)
     * deepStrictEqual(bytes.capacity, 100)
     */
    get capacity() {
        return this._raw.length
    }
    
    /**
     * Forces the length of the vector to new len.
     * @param {number} size - new len.
     * @returns {void}
     * @public
     * @example
     * import { deepStrictEqual } from "assert"
     * 
     * const bytes = new Bytes(100)
     * 
     * deepStrictEqual(bytes.push(0), true)
     * deepStrictEqual(bytes.push(1), true)
     * 
     * bytes.resize(1)
     * deepStrictEqual(bytes.length, 1)
     */
    resize(size) {
        this._len = size
    }
    
    /**
     * Extracts a slice containing the range vector.
     * @param {?number} start - range start index.
     * @param {?number} end - range end index.
     * @returns {Buffer}
     * @public
     * @example
     * import { deepStrictEqual } from "assert"
     * 
     * const bytes = new Bytes(100)
     * 
     * deepStrictEqual(bytes.push(0), true)
     * deepStrictEqual(bytes.push(1), true)
     * deepStrictEqual(bytes.slice(0, 1).length, 1)
     */
    slice(start = 0, end = this._len) {
        return this._raw.subarray(start, end)
    }
    
    /**
     * Copys all the elements of other into Self, 
     * leaving other empty.
     * @param {Buffer} slice
     * @returns {void}
     * @public
     */
    append(slice) {
        const count = this._len + slice.length
        if (count >= this._max) this._expand(count - this._max)
        slice.copy(this._raw, this._len)
        this._len += slice.length
    }
}

/**
 * A trait for values that provide sequential write access to bytes.
 * @class
 */
 exports.BytesMut = class BytesMut {
    
    /**
     * @param {?Buffer | Bytes} raw - raw buffer.
     * @constructor
     */
    constructor(raw = Buffer.alloc(0)) {
        this._raw = raw instanceof Bytes ? raw.slice() : raw
        this._offset = 0
    }

    /**
     * Returns the number of elements in the vector, 
     * also referred to as its ‘length’.
     * @returns {number}
     * @public
     * @example
     * import { deepStrictEqual } from "assert"
     * 
     * const bytes = new Bytes(100)
     * 
     * deepStrictEqual(bytes.push(0), true)
     * deepStrictEqual(bytes.push(1), true)
     * deepStrictEqual(bytes.length, 2)
     */
     get length() {
        return this._raw.length
    }
    
    /**
     * get raw buffer in self.
     * @returns {Buffer}
     * @public
     */
     get raw() {
        return this._raw
    }
    
    /**
     * Returns the number of bytes that can be written from the current 
     * position until the end of the buffer is reached.
     * @returns {number}
     * @public
     */
    get remaining() {
        return this._raw.length - this._offset
    }
    
    /**
     * get inner cursor offset.
     * @returns {number}
     * @public
     */
    get cursor() {
        return this._offset
    }
    
    /**
     * set inner cursor offset.
     * @param {number} offset
     * @returns {void}
     * @public
     */
    set cursor(offset) {
        this._offset = offset
    }
    
    /**
     * advance the internal cursor of the BytesMut.
     * @param {number} cap
     * @returns {void}
     * @public
     */
    advance(cap) {
        this._offset += cap
    }

    /**
     * @param {number} offset
     * @returns {number | undefined}
     * @public 
     */
    get(offset) {
        this._raw[offset]
    }
    
    /**
     * Gets an unsigned 8 bit integer from BytesMut.
     * @returns {number}
     * @public
     */
    get_u8() {
        const u8 = this._raw[this._offset]
        this._offset += 1
        return u8
    }
    
    /**
     * Gets an unsigned 16 bit integer from BytesMut.
     * @returns {number}
     * @public
     */
    get_u16() {
        const u16 = this._raw.readUInt16BE(this._offset)
        this._offset += 2
        return u16
    }
    
    /**
     * Gets an unsigned 32 bit integer from BytesMut.
     * @returns {number}
     * @public
     */
    get_u32() {
        const u32 = this._raw.readUInt32BE(this._offset)
        this._offset += 4
        return u32
    }
    
    /**
     * Gets an unsigned 64 bit integer from BytesMut.
     * @returns {BigInt}
     * @public
     */
    get_u64() {
        const u64 = this._raw.readBigUInt64BE(this._offset)
        this._offset += 8
        return u64
    }
    
    /**
     * Writes an unsigned 8 bit integer to BytesMut.
     * @param {number} u8
     * @returns {void}
     * @public
     */
    put_u8(u8) {
        this._raw[this._offset] = u8
        this._offset += 1
    }
    
    /**
     * Writes an unsigned 16 bit integer to BytesMut.
     * @param {number} u16
     * @returns {void}
     * @public
     */
    put_u16(u16) {
        this._raw.writeUInt16BE(u16, this._offset)
        this._offset += 2
    }
    
    /**
     * Writes an unsigned 32 bit integer to BytesMut.
     * @param {number} u32
     * @returns {void}
     * @public
     */
    put_u32(u32) {
        this._raw.writeUInt32BE(u32, this._offset)
        this._offset += 4
    }
    
    /**
     * Writes an unsigned 64 bit integer to BytesMut.
     * @param {BigInt} u64
     * @returns {void}
     * @public
     */
    put_u64(u64) {
        this._raw.writeBigUInt64BE(u64, this._offset)
        this._offset += 8
    }
    
    /**
     * Writes an slice to BytesMut.
     * @param {Buffer | Bytes} slice
     * @returns {void}
     * @public
     */
    put(slice) {
        const source = slice instanceof Bytes ? slice.slice() : slice
        source.copy(this._raw, this._offset)
    }
    
    /**
     * Gets an range slice from BytesMut.
     * @param {?number} start - start index.
     * @param {?number} end - end index.
     * @returns {Buffer}
     * @public
     */
    slice(start, end = this._offset) {
        return this._raw.subarray(start, end)
    }
}
