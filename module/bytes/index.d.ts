export declare class Bytes {
    constructor(size?: number);
    push(byte: number): void;
    resize(size: number): void;
    slice(start?: number, end?: number): Buffer;
    append(slice: Buffer): void;

    get raw(): Buffer;
    get length(): number;
    get capacity(): number;
}

export declare class BytesMut {
    constructor(raw?: Buffer | Bytes);
    advance(cap: number): void;
    get(offset: number): number | undefined;
    get_u8(): number;
    get_u16(): number;
    get_u32(): number;
    get_u64(): bigint;
    put(slice: Buffer | Bytes): void;
    put_u8(u8: number): void;
    put_u16(u16: number): void;
    put_u32(u32: number): void;
    put_u64(u64: bigint): void;
    slice(start?: number, end?: number): Buffer;

    get raw(): Buffer;
    get length(): number;
    get remaining(): number;
    get cursor(): number;
    set cursor(offset: number);
}
