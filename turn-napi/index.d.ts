export declare class TurnObserver {
    public async get_password(addr: string, name: string): Promise<string | null>;
}

export enum StunClass {
    Msg = 'msg',
    Channel = 'channel',
}

export interface Ret {
    data: Buffer
    kind: StunClass
    interface: string
    relay: string
}

export declare class TurnProcesser {
    public async process(buf: Buffer, addr: string): Promise<Ret | null>;
}

export declare class TurnService<T extends TurnObserver> {
    constructor(realm: string, externals: string[], observer: T);
    public get_processer(inter: string, external: string): TurnProcesser;
}
