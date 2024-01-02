import grpc from '@grpc/grpc-js'
import loader from '@grpc/proto-loader'

export class Balance {
    constructor(client) {
        this._client = client
    }
}

export async function createBalance(server) {
    return new Balance(grpc
        .loadPackageDefinition(await loadProto('../../protos/balance.proto'))
        .routeguide
        .RouteGuide(server, grpc.credentials.createInsecure()))
}

async function loadProto(path) {
    return await loader.load(path, {
        keepCase: true,
        longs: String,
        enums: String,
        defaults: true,
        oneofs: true
    })
}
