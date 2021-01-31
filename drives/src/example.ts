import { createServer } from "net"
import { pipeline } from "stream"
import { Mysticeti } from "./"

// 触发器服务
enum Trigger {
    Auth = 0
}

// 状态服务
enum Service {
    Get = 1,
    Remove = 2
}

// 请求
interface Request {
    addr: string
}

// 认证请求
//
// * `addr` 客户端地址
// * `username` 用户名
interface AuthRequest {
    addr: string
    username: string
}

// 认证信息
//
// * `password` 密钥
// * `group` 分组ID
interface Auth {
    password: string
    group: number
}

// 节点
//
// * `group` 分组ID
// * `delay` 超时时间
// * `clock` 内部时钟
// * `password` 密钥
// * `ports` 分配端口列表
// * `channels` 分配频道列表
interface Node {
    group: number
    delay: number
    clock: number
    ports: number[]
    channels: number[]
    password: string
}

createServer(socket => {
    const mysticeti = new Mysticeti()
    
    mysticeti.bind(Trigger.Auth, (auth: AuthRequest) => {
        console.log("Auth =: ", auth)
        
        setTimeout(async () => {
            console.log(auth.username, await mysticeti.call(Service.Get, {
                addr: auth.addr
            }))
        }, 1000)
        
        return <Auth>{
            password: process.argv[2],
            group: 0
        }
    })
    
    pipeline(
        socket,
        mysticeti,
        socket,
        console.error
    )
}).listen(8080)